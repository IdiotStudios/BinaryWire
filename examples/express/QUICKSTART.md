# BiWi Express Authentication - Quick Start

## What is this?

A real **Express.js HTTP server** with BiWi binary protocol authentication, demonstrating how JavaScript can use BiWi (powered by Rust) for secure, efficient communication.

## Start the Server

```bash
cd examples/express
node server.js
```

## Open in Browser

Navigate to: **http://localhost:3000**

You'll see a beautiful login/register page powered by BiWi!

## Test Programmatically

```bash
node test-client.js
```

## How It Works

1. **Browser/Client** creates a BiWi message with username/password
2. **JavaScript** encodes it to binary using `biwi-bridge.js`
3. **Express.js** receives the binary data
4. **Server** decodes with BiWi and processes authentication
5. **Server** responds with binary BiWi message
6. **Client** decodes and displays results

## Example: Login Flow

### Client (JavaScript)
```javascript
import { BiWiMessage } from './biwi-bridge.js';

const msg = new BiWiMessage();
msg.setField(1, 'myusername');  // Username
msg.setField(2, 'mypassword');  // Password

const buffer = msg.toBuffer();  // Binary encoding

fetch('/api/biwi/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/biwi' },
  body: buffer
});
```

### Server (Express.js)
```javascript
app.post('/api/biwi/login', (req, res) => {
  const msg = BiWiMessage.fromBuffer(req.body);
  const username = msg.getField(1);
  const password = msg.getField(2);
  
  // Authenticate...
  
  const response = new BiWiMessage();
  response.setField(4, true);  // Success
  response.setField(3, sessionToken);
  
  res.type('application/biwi');
  res.send(response.toBuffer());
});
```

## Files

- **`server.js`** - Express HTTP server with BiWi endpoints
- **`biwi-bridge.js`** - JavaScript BiWi encoder/decoder (calls Rust implementation)
- **`test-client.js`** - Automated API tests
- **`README.md`** - Full documentation

## Features

✅ User registration via BiWi  
✅ User login with session tokens  
✅ Session verification  
✅ Beautiful web UI  
✅ Binary protocol efficiency  
✅ Express.js middleware integration  

## Why BiWi over JSON?

### BiWi (Binary)
- **Size**: ~45 bytes
- **Type-safe**: Explicit type codes
- **Fast**: Efficient encoding
- **Streaming**: Partial message support

### JSON (Text)
- **Size**: ~67 bytes
- **Type inference**: Implicit
- **Parsing**: More overhead
- **Buffering**: Need complete message

**BiWi saves ~33% bandwidth!**

## Security Note

⚠️ This is a demo! In production, always:
- Hash passwords (bcrypt/argon2)
- Use HTTPS
- Implement rate limiting (lets be honest your gonna forget about this anyway)
- Add CSRF protection
- Expire sessions properly

## Questions?

Read the full `README.md` for detailed documentation!
