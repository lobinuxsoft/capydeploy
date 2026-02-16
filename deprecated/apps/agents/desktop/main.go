package main

import (
	"embed"
	"flag"
	"fmt"
	"log"
	"os"

	"github.com/wailsapp/wails/v2"
	"github.com/wailsapp/wails/v2/pkg/options"
	"github.com/wailsapp/wails/v2/pkg/options/assetserver"
	"github.com/wailsapp/wails/v2/pkg/options/linux"
	"github.com/wailsapp/wails/v2/pkg/options/windows"

	"github.com/lobinuxsoft/capydeploy/pkg/version"
)

//go:embed all:frontend/dist
var assets embed.FS

func main() {
	showVersion := flag.Bool("version", false, "Show version information and exit")
	noTray := flag.Bool("no-tray", false, "Disable system tray icon")
	flag.Parse()

	if *showVersion {
		fmt.Println("CapyDeploy Agent", version.Full())
		os.Exit(0)
	}

	app := NewApp()
	app.noTray = *noTray

	err := wails.Run(&options.App{
		Title:     "CapyDeploy Agent",
		Width:     500,
		Height:    700,
		MinWidth:  400,
		MinHeight: 500,
		AssetServer: &assetserver.Options{
			Assets: assets,
		},
		BackgroundColour: &options.RGBA{R: 26, G: 26, B: 46, A: 1},
		OnStartup:        app.startup,
		OnShutdown:       app.shutdown,
		Bind: []interface{}{
			app,
		},
		Windows: &windows.Options{
			WebviewIsTransparent: false,
			WindowIsTranslucent:  false,
			DisableWindowIcon:    false,
		},
		Linux: &linux.Options{
			WindowIsTranslucent: false,
		},
	})

	if err != nil {
		log.Fatal(err)
	}
}
