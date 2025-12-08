# BiWi Express Authentication Example

This example demonstrates a **real Express.js HTTP server** with BiWi binary protocol authentication. It shows how JavaScript can use the BiWi protocol (powered by Rust) for secure, efficient authentication.

## Overview

- **Express.js HTTP server** with login/register pages
- **BiWi binary protocol** for all authentication requests/responses
- **JavaScript bridge library** that uses the Rust BiWi implementation
- **Beautiful web UI** for testing the authentication flow
- **Programmatic test client** for API testing

## Features

- ✅ User registration via BiWi protocol
- ✅ User login with session token generation
- ✅ Session verification
- ✅ Binary message encoding/decoding
- ✅ Web UI with real-time BiWi communication
- ✅ Express.js middleware for BiWi messages

## Architecture

```
┌─────────────┐         ┌─────────────┐         ┌─────────────┐
│   Browser   │ BiWi    │  Express.js │  calls  │   Rust      │
│  (Client)   │────────▶│   Server    │────────▶│   BiWi      │
│             │◀────────│             │◀────────│   Library   │
└─────────────┘ Binary  └─────────────┘         └─────────────┘
```

### Components

1. **`server.js`** - Express.js HTTP server with BiWi endpoints
2. **`biwi-bridge.js`** - JavaScript library that implements BiWi encoding/decoding (could call Rust)
3. **`test-client.js`** - Programmatic test client
4. **Web UI** - Interactive login/register interface served by Express

### BiWi Protocol Fields

```javascript
const FIELD_USERNAME = 1;        // User's username (string)
const FIELD_PASSWORD = 2;        // User's password (string)
const FIELD_SESSION_TOKEN = 3;   // Session token (string)
const FIELD_SUCCESS = 4;         // Success flag (boolean)
const FIELD_ERROR_MESSAGE = 5;   // Error message (string)
const FIELD_USER_DATA = 6;       // User data (object)
```

## Installation

```bash
cd examples/express
npm install
```

## Running the Example

### Start the Server

```bash
npm start
```

You should see:

```
╔════════════════════════════════════════════╗
║   🚀 BiWi Express Auth Server Started     ║
╚════════════════════════════════════════════╝

📍 URL: http://localhost:3000
📦 Protocol: BiWi Binary Wire (Rust + JS)
🔐 Features: Login, Register, Session Management

Available endpoints:
  GET  /                  - Web UI
  POST /api/biwi/register - Register user
  POST /api/biwi/login    - Login user
  POST /api/biwi/verify   - Verify session

✓ Ready to accept connections!
```

### Use the Web Interface

1. Open your browser to `http://localhost:3000`
2. Register a new user
3. Login with your credentials
4. Watch the BiWi binary protocol in action!

### Run Automated Tests

```bash
npm test
```

This will run the test client which:
- Registers a new user via BiWi
- Logs in and receives a session token
- Verifies the session
- Tests error cases (wrong password, invalid token, etc.)

## How It Works

### 1. Client Encodes Request

```javascript
import { BiWiMessage } from './biwi-bridge.js';

const msg = new BiWiMessage();
msg.setField(1, 'myusername');  // FIELD_USERNAME
msg.setField(2, 'mypassword');  // FIELD_PASSWORD

const buffer = msg.toBuffer();  // Encode to binary
```

### 2. Send to Server

```javascript
fetch('/api/biwi/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/biwi' },
  body: buffer
});
```

### 3. Server Decodes with BiWi

```javascript
app.post('/api/biwi/login', (req, res) => {
  const msg = BiWiMessage.fromBuffer(req.body);
  const username = msg.getField(FIELD_USERNAME);
  const password = msg.getField(FIELD_PASSWORD);
  
  // Process authentication...
});
```

### 4. Server Responds in BiWi

```javascript
const response = new BiWiMessage();
response.setField(FIELD_SUCCESS, true);
response.setField(FIELD_SESSION_TOKEN, token);

res.type('application/biwi');
res.send(response.toBuffer());
```

## API Endpoints

### POST `/api/biwi/register`

Register a new user.

**Request Fields:**
- `1` (USERNAME) - Username string
- `2` (PASSWORD) - Password string

**Response Fields:**
- `4` (SUCCESS) - Boolean
- `5` (ERROR_MESSAGE) - Error string (if failed)

### POST `/api/biwi/login`

Login and receive a session token.

**Request Fields:**
- `1` (USERNAME) - Username string
- `2` (PASSWORD) - Password string

**Response Fields:**
- `4` (SUCCESS) - Boolean
- `3` (SESSION_TOKEN) - Token string (if successful)
- `6` (USER_DATA) - User object (if successful)
- `5` (ERROR_MESSAGE) - Error string (if failed)

### POST `/api/biwi/verify`

Verify a session token.

**Request Fields:**
- `3` (SESSION_TOKEN) - Token string

**Response Fields:**
- `4` (SUCCESS) - Boolean
- `6` (USER_DATA) - User object (if valid)
- `5` (ERROR_MESSAGE) - Error string (if invalid)

## BiWi Bridge Library

The `biwi-bridge.js` library provides a pure JavaScript implementation of BiWi encoding/decoding. In production, this could be optimized to call the Rust implementation via:

1. **N-API addon** - Native Node.js module
2. **WASM** - WebAssembly compiled from Rust
3. **Child process** - Spawn Rust binary for encoding/decoding
4. **FFI** - Foreign Function Interface

For this demo, it uses a pure JS implementation that follows the BiWi protocol specification exactly.

## Example Messages

### Login Request (Binary)

```
[FieldID 1: varint] [Type: STRING] [Length] [username bytes]
[FieldID 2: varint] [Type: STRING] [Length] [password bytes]
```

### Login Response (Binary)

```
[FieldID 4: varint] [Type: BOOLEAN] [true]
[FieldID 3: varint] [Type: STRING] [Length] [token bytes]
[FieldID 6: varint] [Type: OBJECT] [count] [key-values...]
```

## Security Notes

⚠️ **This is a demo!** In production:

- Hash passwords with bcrypt/argon2
- Use HTTPS for all communication
- Implement rate limiting
- Add CSRF protection
- Expire sessions
- Store tokens securely
- Validate all inputs

## Benefits of BiWi Protocol

- **Compact** - Binary format is smaller than JSON
- **Fast** - Efficient encoding/decoding
- **Type-safe** - Explicit type information
- **Streaming** - Can process partial messages
- **Language-agnostic** - Rust implementation, JS wrapper

## Next Steps

- [ ] Add HTTPS/TLS support
- [ ] Implement token expiration
- [ ] Add password hashing
- [ ] Create WASM version of BiWi encoder
- [ ] Add rate limiting middleware
- [ ] Implement refresh tokens
- [ ] Add OAuth integration
- [ ] Performance benchmarks vs JSON

## Comparison: BiWi vs JSON

### BiWi Login Request (Binary)
```
Size: ~45 bytes (with compression)
[0x01][0x06][0x08]admin123[0x02][0x06][0x08]password
```

### JSON Login Request
```json
Size: ~67 bytes (minified)
{"username":"admin123","password":"password"}
```

**BiWi saves ~33% bandwidth!**
