<div align="center">
  <br>
  <h1>Accounts for COSMIC™</h1>

  <p>A comprehensive online account management system for the COSMIC desktop environment</p>

  ![window-light.png](https://raw.githubusercontent.com/cosmic-utils/accounts/refs/heads/main/accounts-ui/resources/screenshots/welcome-light.png#gh-light-mode-only)
  ![window-dark.png](https://raw.githubusercontent.com/cosmic-utils/accounts/refs/heads/main/accounts-ui/resources/screenshots/welcome-dark.png#gh-dark-mode-only)
</div>


## Building

### Prerequisites

Install the required system libraries:

```bash
# Ubuntu/Debian
sudo apt install libssl-dev pkg-config libdbus-1-dev libsecret-1-dev libxkbcommon-dev libwayland-dev

# Fedora
sudo dnf install openssl-devel dbus-devel libsecret-devel libxkbcommon-devel wayland-devel

# Arch
sudo pacman -S openssl pkgconf dbus libsecret libxkbcommon wayland
```

### Install

```bash
sudo just install
```

This installs:
- `/usr/bin/accounts-daemon` — background D-Bus service
- `/usr/bin/accounts-ui` — the UI application
- `/usr/lib/systemd/user/cosmic-accounts.service` — systemd user unit
- `/etc/accounts/providers/` — OAuth2 provider configs
- `/usr/share/applications/dev.edfloreshz.Accounts.desktop` — app launcher

### OAuth2 Credentials

Before authenticating, you must obtain real OAuth2 credentials and add them to the provider configs. The installed configs contain placeholders that will cause an `invalid_client` error from Google/Microsoft.

#### Google

1. Go to the [Google Cloud Console](https://console.cloud.google.com/)
2. Create or select a project
3. **APIs & Services → OAuth consent screen** — configure the consent screen
4. **APIs & Services → Credentials → Create Credentials → OAuth 2.0 Client ID**
   - Application type: **Web application**
   - Authorized redirect URIs: `http://localhost:8080/callback`
5. Copy the **Client ID** and **Client Secret**
6. Update the config:
   ```bash
   sudo nano /etc/accounts/providers/google.toml
   ```

#### Microsoft

1. Go to the [Azure Portal](https://portal.azure.com/)
2. **Azure Active Directory → App registrations → New registration**
   - Redirect URI: `http://localhost:8080/callback` (Web)
3. Copy the **Application (client) ID**
4. **Certificates & secrets → New client secret** — copy the secret value
5. Update the config:
   ```bash
   sudo nano /etc/accounts/providers/microsoft.toml
   ```

After updating credentials, restart the daemon:

```bash
systemctl --user restart cosmic-accounts
```

### Running

Start the daemon and launch the UI in one step:

```bash
just start
```

Or manually:

```bash
# Start the daemon
just start-daemon

# Launch the UI
/usr/bin/accounts-ui
```

To check daemon status or view logs:

```bash
just status
just logs
```

## Components
**`accounts/` (Core Library)**
- `Account`, `Provider`, and `Credential` models
- Service abstraction layer for different services
- D-Bus client proxy for communication

**`accounts-daemon/`**
- D-Bus service implementation (`dev.edfloreshz.Accounts`)
- OAuth2 authentication manager
- Secure credential storage
- Provider configuration management
- Integrated HTTP callback server

**`accounts-ui/`**
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

- [GitHub Issues](https://github.com/cosmic-utils/accounts/issues)
- [COSMIC Discord](https://discord.gg/cosmic-desktop)
- [System76 Community](https://chat.pop-os.org/)
