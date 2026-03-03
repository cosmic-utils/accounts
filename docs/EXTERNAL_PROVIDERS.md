# Custom OAuth Provider Setup

Because the official COSMIC Online Accounts apps are currently in the testing phase, only pre-approved developer accounts can log in using the default credentials.

If you are a tester or a user who wants to use the app today, you can easily provide your own OAuth credentials.

## 1. Create your own OAuth Application

### Google
1. Go to the [Google Cloud Console](https://console.cloud.google.com/).
2. Create a new project.
3. Search for "APIs & Services" > "Credentials".
4. Click "Create Credentials" > "OAuth client ID".
5. Choose "Web application" (even though this is a desktop app, we use a local callback).
6. Add Authorized Redirect URIs: `http://127.0.0.1:8080/callback`.
7. Note your **Client ID** and **Client Secret**.
8. Enable the following APIs for your project:
   - Google Calendar API
   - People API
   - Tasks API
   - Gmail API

### Microsoft
1. Go to the [Azure Portal](https://portal.azure.com/).
2. Search for "App registrations" and click "New registration".
3. Name it "COSMIC Accounts" and select "Accounts in any organizational directory (Any Microsoft Entra ID tenant - Multitenant) and personal Microsoft accounts".
4. In "Authentication", add a "Web" platform with Redirect URI: `http://127.0.0.1:8080/callback`.
5. Under "Certificates & secrets", create a new "Client secret".
6. Note your **Application (client) ID** and the **Secret Value**.

---

## 2. Apply your Credentials

You can apply your credentials in two ways:

### Option A: Configuration Files (Recommended)
Create a provider TOML file in your user config directory:
`~/.config/cosmic/accounts/providers/google.toml` (or `microsoft.toml`)

**Example `google.toml`:**
```toml
[provider]
client_id = "your-id.googleusercontent.com"
client_secret = "your-secret"
auth_url = "https://accounts.google.com/o/oauth2/v2/auth"
token_url = "https://www.googleapis.com/oauth2/v3/token"
redirect_uri = "http://127.0.0.1:8080/callback"
scopes = [
    "https://www.googleapis.com/auth/userinfo.email",
    "https://www.googleapis.com/auth/userinfo.profile",
    "https://www.googleapis.com/auth/calendar",
    "https://www.googleapis.com/auth/contacts",
    "https://www.googleapis.com/auth/tasks"
]
```

### Option B: Environment Variables
For quick testing, you can run the daemon with environment variables:

```bash
COSMIC_ACCOUNTS_GOOGLE_CLIENT_ID="your-id" 
COSMIC_ACCOUNTS_GOOGLE_CLIENT_SECRET="your-secret" 
accounts-daemon
```

---

## Technical Details for Contributors
The daemon searches for configurations in this order:
1. `$XDG_CONFIG_HOME/cosmic/accounts/providers/`
2. `/etc/cosmic/accounts/providers/`
3. `/usr/share/cosmic/accounts/providers/`
4. Local development path (`accounts-daemon/data/providers/`)
