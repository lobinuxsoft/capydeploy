# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| Latest | Yes |
| Older | No |

As an early-stage project, only the latest version receives updates.

## Reporting a Concern

If you discover a potential security issue, please report it responsibly:

1. **Do NOT** open a public issue
2. **Do** contact the maintainer privately via [GitHub Discussions](https://github.com/lobinuxsoft/capydeploy/discussions) (private message) or email
3. Include as much detail as possible to help reproduce and understand the issue

## Response Timeline

- **Acknowledgment**: Within 72 hours
- **Initial assessment**: Within 1 week
- **Resolution timeline**: Depends on complexity, communicated after assessment

## Scope

This policy applies to:
- The CapyDeploy application code
- WebSocket connection handling
- mDNS discovery and pairing
- Steam shortcut management
- Embedded binaries

**Out of scope**:
- Third-party dependencies (Wails upstream) - report to their respective projects
- User-provided device configurations
- Remote device security

## Security Considerations

This application handles:
- **Pairing tokens**: Stored locally in config file with restricted permissions
- **Remote connections**: WebSocket to mDNS-discovered devices on the local network
- **Steam shortcuts**: Modification of Steam configuration on remote devices

### Best Practices for Users

1. Only pair with devices you trust on your local network
2. Keep your config file secure (default: user-only permissions)
3. Keep the application updated

## Recognition

Contributors who responsibly report valid issues will be credited in release notes (unless they prefer anonymity).
