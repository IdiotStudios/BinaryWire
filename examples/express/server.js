/**
 * BiWi Express Authentication Server
 * Demonstrates using BiWi protocol for secure authentication in an HTTP server
 */

import express from 'express';
import { BiWiMessage } from './biwi-bridge.js';

const app = express();
const PORT = 3000;

// In-memory user database (demo purposes)
const users = new Map();
const sessions = new Map();

// Field IDs for BiWi protocol
const FIELD_USERNAME = 1;
const FIELD_PASSWORD = 2;
const FIELD_SESSION_TOKEN = 3;
const FIELD_SUCCESS = 4;
const FIELD_ERROR_MESSAGE = 5;
const FIELD_USER_DATA = 6;

// Middleware to parse BiWi binary messages
app.use('/api/biwi', express.raw({ type: 'application/biwi', limit: '10mb' }));

// Middleware to parse JSON for web interface
app.use(express.json());
app.use(express.urlencoded({ extended: true }));

// Serve static login page
app.get('/', (req, res) => {
  res.send(`
<!DOCTYPE html>
<html>
<head>
  <title>BiWi Auth Demo</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
      min-height: 100vh;
      display: flex;
      justify-content: center;
      align-items: center;
      padding: 20px;
    }
    .container {
      background: white;
      border-radius: 10px;
      box-shadow: 0 10px 40px rgba(0,0,0,0.2);
      padding: 40px;
      max-width: 400px;
      width: 100%;
    }
    h1 {
      color: #333;
      margin-bottom: 10px;
      font-size: 28px;
    }
    .subtitle {
      color: #666;
      margin-bottom: 30px;
      font-size: 14px;
    }
    .protocol-badge {
      display: inline-block;
      background: #667eea;
      color: white;
      padding: 4px 12px;
      border-radius: 12px;
      font-size: 11px;
      font-weight: bold;
      margin-bottom: 20px;
    }
    .form-group {
      margin-bottom: 20px;
    }
    label {
      display: block;
      margin-bottom: 8px;
      color: #555;
      font-weight: 500;
      font-size: 14px;
    }
    input {
      width: 100%;
      padding: 12px;
      border: 2px solid #e0e0e0;
      border-radius: 6px;
      font-size: 14px;
      transition: border-color 0.3s;
    }
    input:focus {
      outline: none;
      border-color: #667eea;
    }
    button {
      width: 100%;
      padding: 14px;
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
      color: white;
      border: none;
      border-radius: 6px;
      font-size: 16px;
      font-weight: 600;
      cursor: pointer;
      transition: transform 0.2s, box-shadow 0.2s;
    }
    button:hover {
      transform: translateY(-2px);
      box-shadow: 0 5px 15px rgba(102, 126, 234, 0.4);
    }
    button:active {
      transform: translateY(0);
    }
    .message {
      padding: 12px;
      margin-bottom: 20px;
      border-radius: 6px;
      font-size: 14px;
      display: none;
    }
    .message.success {
      background: #d4edda;
      color: #155724;
      border: 1px solid #c3e6cb;
    }
    .message.error {
      background: #f8d7da;
      color: #721c24;
      border: 1px solid #f5c6cb;
    }
    .tabs {
      display: flex;
      margin-bottom: 30px;
      border-bottom: 2px solid #e0e0e0;
    }
    .tab {
      flex: 1;
      padding: 12px;
      text-align: center;
      cursor: pointer;
      color: #666;
      font-weight: 500;
      transition: color 0.3s;
    }
    .tab.active {
      color: #667eea;
      border-bottom: 3px solid #667eea;
      margin-bottom: -2px;
    }
    .tab-content {
      display: none;
    }
    .tab-content.active {
      display: block;
    }
    .user-info {
      background: #f8f9fa;
      padding: 20px;
      border-radius: 6px;
      margin-top: 20px;
    }
    .user-info h3 {
      margin-bottom: 10px;
      color: #333;
    }
    .user-info p {
      color: #666;
      margin: 5px 0;
      font-size: 14px;
    }
    .tech-info {
      margin-top: 30px;
      padding-top: 20px;
      border-top: 1px solid #e0e0e0;
      font-size: 12px;
      color: #999;
    }
    .code {
      background: #f5f5f5;
      padding: 2px 6px;
      border-radius: 3px;
      font-family: 'Courier New', monospace;
    }
  </style>
</head>
<body>
  <div class="container">
    <h1>🔐 BiWi Auth</h1>
    <p class="subtitle">Binary Wire Protocol Authentication</p>
    <span class="protocol-badge">POWERED BY RUST + JAVASCRIPT</span>
    
    <div id="message" class="message"></div>

    <div class="tabs">
      <div class="tab active" onclick="switchTab('login')">Login</div>
      <div class="tab" onclick="switchTab('register')">Register</div>
    </div>

    <div id="login-tab" class="tab-content active">
      <form onsubmit="handleLogin(event)">
        <div class="form-group">
          <label>Username</label>
          <input type="text" id="login-username" required autocomplete="username">
        </div>
        <div class="form-group">
          <label>Password</label>
          <input type="password" id="login-password" required autocomplete="current-password">
        </div>
        <button type="submit">Login with BiWi</button>
      </form>
    </div>

    <div id="register-tab" class="tab-content">
      <form onsubmit="handleRegister(event)">
        <div class="form-group">
          <label>Username</label>
          <input type="text" id="register-username" required autocomplete="username">
        </div>
        <div class="form-group">
          <label>Password</label>
          <input type="password" id="register-password" required autocomplete="new-password">
        </div>
        <button type="submit">Register with BiWi</button>
      </form>
    </div>

    <div id="user-info" class="user-info" style="display: none;">
      <h3>✓ Authenticated</h3>
      <p><strong>Username:</strong> <span id="user-username"></span></p>
      <p><strong>Session Token:</strong> <span class="code" id="user-token"></span></p>
      <p><strong>Protocol:</strong> BiWi Binary Wire</p>
      <button onclick="logout()" style="margin-top: 15px;">Logout</button>
    </div>

    <div class="tech-info">
      <p><strong>How it works:</strong></p>
      <p>• JavaScript encodes auth data using BiWi protocol</p>
      <p>• Binary message sent to Express.js server</p>
      <p>• Rust library validates and processes (via bridge)</p>
      <p>• Response encoded back to client in BiWi format</p>
    </div>
  </div>

  <script type="module">
    import { BiWiMessage } from './biwi-bridge-client.js';

    window.BiWiMessage = BiWiMessage;

    window.switchTab = function(tab) {
      document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
      document.querySelectorAll('.tab-content').forEach(t => t.classList.remove('active'));
      
      if (tab === 'login') {
        document.querySelectorAll('.tab')[0].classList.add('active');
        document.getElementById('login-tab').classList.add('active');
      } else {
        document.querySelectorAll('.tab')[1].classList.add('active');
        document.getElementById('register-tab').classList.add('active');
      }
    };

    window.showMessage = function(message, type) {
      const msgEl = document.getElementById('message');
      msgEl.textContent = message;
      msgEl.className = 'message ' + type;
      msgEl.style.display = 'block';
      setTimeout(() => {
        msgEl.style.display = 'none';
      }, 5000);
    };

    window.handleLogin = async function(event) {
      event.preventDefault();
      
      const username = document.getElementById('login-username').value;
      const password = document.getElementById('login-password').value;

      try {
        const msg = new BiWiMessage();
        msg.setField(1, username); // FIELD_USERNAME
        msg.setField(2, password); // FIELD_PASSWORD

        const buffer = msg.toBuffer();

        const response = await fetch('/api/biwi/login', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/biwi'
          },
          body: buffer
        });

        const responseBuffer = await response.arrayBuffer();
        const responseMsg = BiWiMessage.fromBuffer(new Uint8Array(responseBuffer));

        if (responseMsg.getField(4)) { // FIELD_SUCCESS
          const token = responseMsg.getField(3); // FIELD_SESSION_TOKEN
          showMessage('Login successful!', 'success');
          
          document.getElementById('user-username').textContent = username;
          document.getElementById('user-token').textContent = token.substring(0, 20) + '...';
          document.getElementById('user-info').style.display = 'block';
          document.getElementById('login-tab').style.display = 'none';
          document.getElementById('register-tab').style.display = 'none';
          document.querySelector('.tabs').style.display = 'none';
        } else {
          showMessage(responseMsg.getField(5) || 'Login failed', 'error');
        }
      } catch (error) {
        showMessage('Error: ' + error.message, 'error');
      }
    };

    window.handleRegister = async function(event) {
      event.preventDefault();
      
      const username = document.getElementById('register-username').value;
      const password = document.getElementById('register-password').value;

      try {
        const msg = new BiWiMessage();
        msg.setField(1, username); // FIELD_USERNAME
        msg.setField(2, password); // FIELD_PASSWORD

        const buffer = msg.toBuffer();

        const response = await fetch('/api/biwi/register', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/biwi'
          },
          body: buffer
        });

        const responseBuffer = await response.arrayBuffer();
        const responseMsg = BiWiMessage.fromBuffer(new Uint8Array(responseBuffer));

        if (responseMsg.getField(4)) { // FIELD_SUCCESS
          showMessage('Registration successful! Please login.', 'success');
          switchTab('login');
        } else {
          showMessage(responseMsg.getField(5) || 'Registration failed', 'error');
        }
      } catch (error) {
        showMessage('Error: ' + error.message, 'error');
      }
    };

    window.logout = function() {
      document.getElementById('user-info').style.display = 'none';
      document.getElementById('login-tab').style.display = 'block';
      document.getElementById('register-tab').style.display = 'block';
      document.querySelector('.tabs').style.display = 'flex';
      document.getElementById('login-username').value = '';
      document.getElementById('login-password').value = '';
      showMessage('Logged out successfully', 'success');
    };
  </script>
</body>
</html>
  `);
});

// Serve BiWi bridge client library
app.get('/biwi-bridge-client.js', (req, res) => {
  res.type('application/javascript');
  res.sendFile(new URL('./biwi-bridge.js', import.meta.url).pathname);
});

// BiWi API: Register endpoint
app.post('/api/biwi/register', (req, res) => {
  try {
    const msg = BiWiMessage.fromBuffer(req.body);
    
    const username = msg.getField(FIELD_USERNAME);
    const password = msg.getField(FIELD_PASSWORD);

    console.log(`📝 Registration attempt: ${username}`);

    const response = new BiWiMessage();

    if (!username || !password) {
      response.setField(FIELD_SUCCESS, false);
      response.setField(FIELD_ERROR_MESSAGE, 'Username and password required');
    } else if (users.has(username)) {
      response.setField(FIELD_SUCCESS, false);
      response.setField(FIELD_ERROR_MESSAGE, 'Username already exists');
    } else {
      // Register user (in production, hash the password!)
      users.set(username, {
        password,
        createdAt: new Date().toISOString()
      });
      
      response.setField(FIELD_SUCCESS, true);
      console.log(`✓ User registered: ${username}`);
    }

    res.type('application/biwi');
    res.send(response.toBuffer());
  } catch (error) {
    console.error('Registration error:', error);
    const response = new BiWiMessage();
    response.setField(FIELD_SUCCESS, false);
    response.setField(FIELD_ERROR_MESSAGE, 'Server error');
    res.type('application/biwi');
    res.send(response.toBuffer());
  }
});

// BiWi API: Login endpoint
app.post('/api/biwi/login', (req, res) => {
  try {
    const msg = BiWiMessage.fromBuffer(req.body);
    
    const username = msg.getField(FIELD_USERNAME);
    const password = msg.getField(FIELD_PASSWORD);

    console.log(`🔐 Login attempt: ${username}`);

    const response = new BiWiMessage();

    const user = users.get(username);
    
    if (!user || user.password !== password) {
      response.setField(FIELD_SUCCESS, false);
      response.setField(FIELD_ERROR_MESSAGE, 'Invalid username or password');
    } else {
      // Create session token
      const token = Buffer.from(
        `${username}-${Date.now()}-${Math.random().toString(36)}`
      ).toString('base64');
      
      sessions.set(token, { username, createdAt: Date.now() });
      
      response.setField(FIELD_SUCCESS, true);
      response.setField(FIELD_SESSION_TOKEN, token);
      response.setField(FIELD_USER_DATA, {
        username,
        loginTime: new Date().toISOString()
      });
      
      console.log(`✓ User logged in: ${username}`);
    }

    res.type('application/biwi');
    res.send(response.toBuffer());
  } catch (error) {
    console.error('Login error:', error);
    const response = new BiWiMessage();
    response.setField(FIELD_SUCCESS, false);
    response.setField(FIELD_ERROR_MESSAGE, 'Server error');
    res.type('application/biwi');
    res.send(response.toBuffer());
  }
});

// BiWi API: Verify session endpoint
app.post('/api/biwi/verify', (req, res) => {
  try {
    const msg = BiWiMessage.fromBuffer(req.body);
    const token = msg.getField(FIELD_SESSION_TOKEN);

    const response = new BiWiMessage();
    const session = sessions.get(token);

    if (session) {
      response.setField(FIELD_SUCCESS, true);
      response.setField(FIELD_USER_DATA, { username: session.username });
    } else {
      response.setField(FIELD_SUCCESS, false);
      response.setField(FIELD_ERROR_MESSAGE, 'Invalid or expired session');
    }

    res.type('application/biwi');
    res.send(response.toBuffer());
  } catch (error) {
    console.error('Verify error:', error);
    const response = new BiWiMessage();
    response.setField(FIELD_SUCCESS, false);
    response.setField(FIELD_ERROR_MESSAGE, 'Server error');
    res.type('application/biwi');
    res.send(response.toBuffer());
  }
});

// Start server
app.listen(PORT, () => {
  console.log('╔════════════════════════════════════════════╗');
  console.log('║   🚀 BiWi Express Auth Server Started     ║');
  console.log('╚════════════════════════════════════════════╝');
  console.log('');
  console.log(`📍 URL: http://localhost:${PORT}`);
  console.log('📦 Protocol: BiWi Binary Wire (Rust + JS)');
  console.log('🔐 Features: Login, Register, Session Management');
  console.log('');
  console.log('Available endpoints:');
  console.log('  GET  /                  - Web UI');
  console.log('  POST /api/biwi/register - Register user');
  console.log('  POST /api/biwi/login    - Login user');
  console.log('  POST /api/biwi/verify   - Verify session');
  console.log('');
  console.log('✓ Ready to accept connections!');
  console.log('');
});
