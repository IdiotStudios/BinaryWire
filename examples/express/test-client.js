/**
 * Test client for BiWi Express Authentication
 * Demonstrates programmatic usage of the BiWi API
 */

import { BiWiMessage } from './biwi-bridge.js';

const BASE_URL = 'http://localhost:3000';

// Field IDs
const FIELD_USERNAME = 1;
const FIELD_PASSWORD = 2;
const FIELD_SESSION_TOKEN = 3;
const FIELD_SUCCESS = 4;
const FIELD_ERROR_MESSAGE = 5;
const FIELD_USER_DATA = 6;

async function sendBiWiRequest(endpoint, message) {
  const buffer = message.toBuffer();
  
  const response = await fetch(`${BASE_URL}${endpoint}`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/biwi'
    },
    body: buffer
  });

  const responseBuffer = await response.arrayBuffer();
  return BiWiMessage.fromBuffer(Buffer.from(responseBuffer));
}

async function testRegister(username, password) {
  console.log(`\n📝 Testing registration: ${username}`);
  
  const msg = new BiWiMessage();
  msg.setField(FIELD_USERNAME, username);
  msg.setField(FIELD_PASSWORD, password);

  const response = await sendBiWiRequest('/api/biwi/register', msg);
  
  if (response.getField(FIELD_SUCCESS)) {
    console.log('✓ Registration successful');
    return true;
  } else {
    console.log('✗ Registration failed:', response.getField(FIELD_ERROR_MESSAGE));
    return false;
  }
}

async function testLogin(username, password) {
  console.log(`\n🔐 Testing login: ${username}`);
  
  const msg = new BiWiMessage();
  msg.setField(FIELD_USERNAME, username);
  msg.setField(FIELD_PASSWORD, password);

  const response = await sendBiWiRequest('/api/biwi/login', msg);
  
  if (response.getField(FIELD_SUCCESS)) {
    const token = response.getField(FIELD_SESSION_TOKEN);
    const userData = response.getField(FIELD_USER_DATA);
    console.log('✓ Login successful');
    console.log('  Token:', token.substring(0, 30) + '...');
    console.log('  User data:', userData);
    return token;
  } else {
    console.log('✗ Login failed:', response.getField(FIELD_ERROR_MESSAGE));
    return null;
  }
}

async function testVerify(token) {
  console.log(`\n🔍 Testing session verification`);
  
  const msg = new BiWiMessage();
  msg.setField(FIELD_SESSION_TOKEN, token);

  const response = await sendBiWiRequest('/api/biwi/verify', msg);
  
  if (response.getField(FIELD_SUCCESS)) {
    const userData = response.getField(FIELD_USER_DATA);
    console.log('✓ Session valid');
    console.log('  User:', userData.username);
    return true;
  } else {
    console.log('✗ Session invalid:', response.getField(FIELD_ERROR_MESSAGE));
    return false;
  }
}

async function runTests() {
  console.log('╔════════════════════════════════════════════╗');
  console.log('║   BiWi Express Authentication Test        ║');
  console.log('╚════════════════════════════════════════════╝');

  try {
    const testUser = `testuser_${Date.now()}`;
    const testPass = 'secure123';

    // Test 1: Register
    await testRegister(testUser, testPass);

    // Test 2: Login with correct credentials
    const token = await testLogin(testUser, testPass);

    // Test 3: Verify session
    if (token) {
      await testVerify(token);
    }

    // Test 4: Login with wrong password
    console.log(`\n🔐 Testing login with wrong password`);
    await testLogin(testUser, 'wrongpassword');

    // Test 5: Duplicate registration
    console.log(`\n📝 Testing duplicate registration`);
    await testRegister(testUser, testPass);

    // Test 6: Verify invalid token
    console.log(`\n🔍 Testing invalid token`);
    await testVerify('invalid-token-12345');

    console.log('\n✓ All tests completed!');
    console.log('');
    
  } catch (error) {
    console.error('\n✗ Test error:', error.message);
    console.error('  Make sure the server is running: npm start');
  }
}

runTests();
