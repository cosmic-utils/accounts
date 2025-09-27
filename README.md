# COSMIC Accounts

A comprehensive online account management system for the COSMIC desktop environment, providing secure OAuth2 authentication and credential management for various online services.

## Components
**`cosmic-accounts/` (Core Library)**
- `Account`, `Provider`, and `Credential` models
- Service abstraction layer for different capabilities
- D-Bus client proxy for communication

**`cosmic-accounts-daemon/`**
- D-Bus service implementation (`com.system76.CosmicAccounts`)
- OAuth2 authentication manager
- Secure credential storage
- Provider configuration management
- Integrated HTTP callback server

**`cosmic-accounts-gui/`**
- COSMIC desktop application
- Account listing and management
- Provider selection and authentication flow
- Visual account status and controls


## Architecture

### **Authentication System**
- **OAuth2 with PKCE** for enhanced security
- **Automatic token refresh** to maintain valid credentials
- **Built-in callback server** for seamless auth flow
- **CSRF protection** for auth requests

### **Provider Support:**
- **Google** - Gmail, Calendar, Contacts, Drive integration
- **Microsoft** - Outlook, Office 365, OneDrive support
- **Extensible provider system** for easy addition of new services

### **Service Integration**
- **Mail services** (Gmail, Outlook)
- **Calendar synchronization** (Google Calendar, Outlook Calendar)

## Contributing

Contributions are welcome! Please read our [Contributing Guidelines](CONTRIBUTING.md) and [Code of Conduct](CODE_OF_CONDUCT.md).

## Related Projects

- [GNOME Online Accounts](https://gitlab.gnome.org/GNOME/gnome-online-accounts) - Inspiration for this project
- [libcosmic](https://github.com/pop-os/libcosmic) - COSMIC UI toolkit
- [COSMIC Desktop](https://github.com/pop-os/cosmic-epoch) - The COSMIC desktop environment

## Support

- [GitHub Issues](https://github.com/cosmic-utils/cosmic-accounts/issues)
- [COSMIC Discord](https://discord.gg/cosmic-desktop)
- [System76 Community](https://chat.pop-os.org/)
