package server

import (
	"encoding/json"
	"log"
	"net"
	"net/http"
	"sync"
	"time"

	"github.com/google/uuid"
	"github.com/gorilla/websocket"
	"github.com/lobinuxsoft/capydeploy/apps/agents/desktop/auth"
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

// WSServer handles WebSocket connections from the Hub.
type WSServer struct {
	server   *Server
	authMgr  *auth.Manager
	upgrader websocket.Upgrader

	mu           sync.RWMutex
	hubConn      *HubConnection
	onConnect    func(hubID, hubName, hubIP string)
	onDisconnect func()
}

// HubConnection represents an active connection from a Hub.
type HubConnection struct {
	conn       *websocket.Conn
	name       string
	version    string
	hubID      string
	remoteAddr string
	authorized bool
	sendCh     chan []byte
	closeCh    chan struct{}
	closed     bool
	closeMu    sync.Mutex
}

// NewWSServer creates a new WebSocket server.
func NewWSServer(s *Server, authMgr *auth.Manager, onConnect func(string, string, string), onDisconnect func()) *WSServer {
	return &WSServer{
		server:  s,
		authMgr: authMgr,
		upgrader: websocket.Upgrader{
			ReadBufferSize:  1024,
			WriteBufferSize: 1024,
			CheckOrigin: func(r *http.Request) bool {
				return true // Allow all origins for local network
			},
		},
		onConnect:    onConnect,
		onDisconnect: onDisconnect,
	}
}

// DisconnectHub closes the current Hub connection if any.
func (ws *WSServer) DisconnectHub() {
	ws.mu.Lock()
	hub := ws.hubConn
	ws.mu.Unlock()

	if hub != nil && hub.conn != nil {
		hub.conn.Close()
	}
}

// HandleWS handles the WebSocket upgrade and connection.
func (ws *WSServer) HandleWS(w http.ResponseWriter, r *http.Request) {
	// Check if connections are accepted
	if ws.server.cfg.AcceptConnections != nil && !ws.server.cfg.AcceptConnections() {
		http.Error(w, "connections not accepted", http.StatusServiceUnavailable)
		log.Printf("WS: Rejected connection from %s: connections disabled", r.RemoteAddr)
		return
	}

	// Check if already connected
	ws.mu.RLock()
	hasConnection := ws.hubConn != nil
	ws.mu.RUnlock()

	if hasConnection {
		http.Error(w, "hub already connected", http.StatusConflict)
		log.Printf("WS: Rejected connection from %s: hub already connected", r.RemoteAddr)
		return
	}

	// Upgrade connection
	conn, err := ws.upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("WS: Upgrade failed from %s: %v", r.RemoteAddr, err)
		return
	}

	log.Printf("WS: New connection from %s", r.RemoteAddr)

	// Extract IP from remote address (format: "IP:port")
	remoteIP := r.RemoteAddr
	if host, _, err := net.SplitHostPort(r.RemoteAddr); err == nil {
		remoteIP = host
	}

	// Create hub connection
	hub := &HubConnection{
		conn:       conn,
		remoteAddr: remoteIP,
		sendCh:     make(chan []byte, 256),
		closeCh:    make(chan struct{}),
	}

	ws.mu.Lock()
	ws.hubConn = hub
	ws.mu.Unlock()

	// Start goroutines
	go ws.writePump(hub)
	go ws.readPump(hub)
}

// readPump handles incoming messages from the Hub.
func (ws *WSServer) readPump(hub *HubConnection) {
	defer func() {
		ws.closeHub(hub)
	}()

	hub.conn.SetReadLimit(protocol.WSMaxMessageSize)
	hub.conn.SetReadDeadline(time.Now().Add(protocol.WSPongWait))
	hub.conn.SetPongHandler(func(string) error {
		hub.conn.SetReadDeadline(time.Now().Add(protocol.WSPongWait))
		return nil
	})

	for {
		messageType, data, err := hub.conn.ReadMessage()
		if err != nil {
			if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseAbnormalClosure) {
				log.Printf("WS: Read error: %v", err)
			}
			return
		}

		switch messageType {
		case websocket.TextMessage:
			ws.handleTextMessage(hub, data)
		case websocket.BinaryMessage:
			ws.handleBinaryMessage(hub, data)
		}
	}
}

// writePump handles outgoing messages to the Hub.
func (ws *WSServer) writePump(hub *HubConnection) {
	ticker := time.NewTicker(protocol.WSPingPeriod)
	defer func() {
		ticker.Stop()
		hub.conn.Close()
	}()

	for {
		select {
		case message, ok := <-hub.sendCh:
			hub.conn.SetWriteDeadline(time.Now().Add(protocol.WSWriteWait))
			if !ok {
				hub.conn.WriteMessage(websocket.CloseMessage, []byte{})
				return
			}

			if err := hub.conn.WriteMessage(websocket.TextMessage, message); err != nil {
				log.Printf("WS: Write error: %v", err)
				return
			}

		case <-ticker.C:
			hub.conn.SetWriteDeadline(time.Now().Add(protocol.WSWriteWait))
			if err := hub.conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				return
			}

		case <-hub.closeCh:
			return
		}
	}
}

// handleTextMessage processes JSON messages from the Hub.
func (ws *WSServer) handleTextMessage(hub *HubConnection, data []byte) {
	var msg protocol.Message
	if err := json.Unmarshal(data, &msg); err != nil {
		log.Printf("WS: Invalid message: %v", err)
		ws.sendError(hub, "", protocol.WSErrCodeBadRequest, "invalid message format")
		return
	}

	if ws.server.cfg.Verbose {
		log.Printf("WS: Received %s (id=%s)", msg.Type, msg.ID)
	}

	// Route message by type
	switch msg.Type {
	case protocol.MsgTypeHubConnected:
		ws.handleHubConnected(hub, &msg)
	case protocol.MsgTypePairConfirm:
		ws.handlePairConfirm(hub, &msg)
	case protocol.MsgTypePing:
		ws.handlePing(hub, &msg)
	case protocol.MsgTypeGetInfo:
		ws.handleGetInfo(hub, &msg)
	case protocol.MsgTypeGetConfig:
		ws.handleGetConfig(hub, &msg)
	case protocol.MsgTypeGetSteamUsers:
		ws.handleGetSteamUsers(hub, &msg)
	case protocol.MsgTypeListShortcuts:
		ws.handleListShortcuts(hub, &msg)
	case protocol.MsgTypeCreateShortcut:
		ws.handleCreateShortcut(hub, &msg)
	case protocol.MsgTypeDeleteShortcut:
		ws.handleDeleteShortcut(hub, &msg)
	case protocol.MsgTypeDeleteGame:
		ws.handleDeleteGame(hub, &msg)
	case protocol.MsgTypeApplyArtwork:
		ws.handleApplyArtwork(hub, &msg)
	case protocol.MsgTypeRestartSteam:
		ws.handleRestartSteam(hub, &msg)
	case protocol.MsgTypeInitUpload:
		ws.handleInitUpload(hub, &msg)
	case protocol.MsgTypeUploadChunk:
		ws.handleUploadChunk(hub, &msg)
	case protocol.MsgTypeCompleteUpload:
		ws.handleCompleteUpload(hub, &msg)
	case protocol.MsgTypeCancelUpload:
		ws.handleCancelUpload(hub, &msg)
	default:
		log.Printf("WS: Unknown message type: %s", msg.Type)
		ws.sendError(hub, msg.ID, protocol.WSErrCodeNotImplemented, "unknown message type")
	}
}

// handleBinaryMessage processes binary data (upload chunks).
func (ws *WSServer) handleBinaryMessage(hub *HubConnection, data []byte) {
	// Binary messages are upload chunks
	// Format: [4 bytes: header length][header JSON][chunk data]
	if len(data) < 4 {
		log.Printf("WS: Binary message too short")
		return
	}

	headerLen := int(data[0])<<24 | int(data[1])<<16 | int(data[2])<<8 | int(data[3])
	if len(data) < 4+headerLen {
		log.Printf("WS: Binary message header incomplete")
		return
	}

	var header struct {
		ID       string `json:"id"`
		UploadID string `json:"uploadId"`
		FilePath string `json:"filePath"`
		Offset   int64  `json:"offset"`
		Checksum string `json:"checksum,omitempty"`
	}

	if err := json.Unmarshal(data[4:4+headerLen], &header); err != nil {
		log.Printf("WS: Invalid binary header: %v", err)
		return
	}

	chunkData := data[4+headerLen:]
	ws.handleBinaryChunk(hub, header.ID, header.UploadID, header.FilePath, header.Offset, header.Checksum, chunkData)
}

// closeHub closes the hub connection and notifies.
func (ws *WSServer) closeHub(hub *HubConnection) {
	hub.closeMu.Lock()
	if hub.closed {
		hub.closeMu.Unlock()
		return
	}
	hub.closed = true
	hub.closeMu.Unlock()

	close(hub.closeCh)
	hub.conn.Close()

	ws.mu.Lock()
	if ws.hubConn == hub {
		ws.hubConn = nil
	}
	ws.mu.Unlock()

	log.Printf("WS: Hub disconnected (%s)", hub.name)
	if ws.onDisconnect != nil {
		ws.onDisconnect()
	}
}

// send sends a message to the hub.
func (ws *WSServer) send(hub *HubConnection, msg *protocol.Message) {
	data, err := json.Marshal(msg)
	if err != nil {
		log.Printf("WS: Marshal error: %v", err)
		return
	}

	hub.closeMu.Lock()
	if hub.closed {
		hub.closeMu.Unlock()
		return
	}
	hub.closeMu.Unlock()

	select {
	case hub.sendCh <- data:
	default:
		log.Printf("WS: Send buffer full, dropping message")
	}
}

// sendError sends an error message.
func (ws *WSServer) sendError(hub *HubConnection, id string, code int, message string) {
	if id == "" {
		id = uuid.New().String()
	}
	ws.send(hub, protocol.NewErrorMessage(id, code, message))
}

// SendEvent sends a push event to the connected hub.
func (ws *WSServer) SendEvent(msgType protocol.MessageType, payload any) {
	ws.mu.RLock()
	hub := ws.hubConn
	ws.mu.RUnlock()

	if hub == nil {
		return
	}

	msg, err := protocol.NewMessage(uuid.New().String(), msgType, payload)
	if err != nil {
		log.Printf("WS: Failed to create event: %v", err)
		return
	}

	ws.send(hub, msg)
}

// IsConnected returns true if a hub is connected.
func (ws *WSServer) IsConnected() bool {
	ws.mu.RLock()
	defer ws.mu.RUnlock()
	return ws.hubConn != nil
}

// GetConnectedHub returns the name of the connected hub, or empty if none.
func (ws *WSServer) GetConnectedHub() string {
	ws.mu.RLock()
	defer ws.mu.RUnlock()
	if ws.hubConn != nil {
		return ws.hubConn.name
	}
	return ""
}
