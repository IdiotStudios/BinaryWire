/**
 * BiWi Bridge - JavaScript library that calls Rust BiWi implementation
 * This uses Node.js child_process to execute Rust binaries for encoding/decoding
 */

import { spawn } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import fs from 'fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Path to Rust BiWi tools
const RUST_BIN_PATH = join(__dirname, '../../target/release');

/**
 * BiWi Value types matching Rust implementation
 */
export const BiWiTypes = {
  NULL: 0x00,
  BOOLEAN: 0x01,
  INT32: 0x02,
  INT64: 0x03,
  FLOAT32: 0x04,
  FLOAT64: 0x05,
  STRING: 0x06,
  BINARY: 0x07,
  ARRAY: 0x08,
  OBJECT: 0x09,
  CHUNK_START: 0x0A,
  CHUNK_DATA: 0x0B,
  CHUNK_END: 0x0C,
};

/**
 * Pure JavaScript implementation of BiWi encoding/decoding
 * (Simplified version - could call Rust for production use)
 */
export class BiWiMessage {
  constructor() {
    this.fields = new Map();
  }

  setField(fieldId, value) {
    this.fields.set(fieldId, value);
    return this;
  }

  getField(fieldId) {
    return this.fields.get(fieldId);
  }

  hasField(fieldId) {
    return this.fields.has(fieldId);
  }

  /**
   * Encode to Buffer using simplified BiWi format
   */
  toBuffer() {
    const chunks = [];

    for (const [fieldId, value] of this.fields) {
      // Encode field ID as varint
      chunks.push(this._encodeVarInt(fieldId));
      // Encode value
      chunks.push(this._encodeValue(value));
    }

    return Buffer.concat(chunks);
  }

  /**
   * Decode from Buffer
   */
  static fromBuffer(buffer) {
    const msg = new BiWiMessage();
    let offset = 0;

    while (offset < buffer.length) {
      // Decode field ID
      const { value: fieldId, length: idLen } = BiWiMessage._decodeVarInt(buffer, offset);
      offset += idLen;

      // Decode value
      const { value, length: valueLen } = BiWiMessage._decodeValue(buffer, offset);
      offset += valueLen;

      msg.setField(fieldId, value);
    }

    return msg;
  }

  /**
   * Encode varint (variable-length integer)
   */
  _encodeVarInt(value) {
    const bytes = [];
    let v = value;
    while (v >= 0x80) {
      bytes.push((v & 0x7f) | 0x80);
      v >>>= 7;
    }
    bytes.push(v & 0x7f);
    return Buffer.from(bytes);
  }

  /**
   * Decode varint from buffer at offset
   */
  static _decodeVarInt(buffer, offset) {
    let value = 0;
    let shift = 0;
    let length = 0;

    while (offset + length < buffer.length) {
      const byte = buffer[offset + length];
      length++;
      value |= (byte & 0x7f) << shift;
      if ((byte & 0x80) === 0) {
        return { value, length };
      }
      shift += 7;
    }

    throw new Error('Invalid varint');
  }

  /**
   * Encode a value based on its type
   */
  _encodeValue(value) {
    if (value === null || value === undefined) {
      return Buffer.from([BiWiTypes.NULL]);
    }

    if (typeof value === 'boolean') {
      return Buffer.from([value ? BiWiTypes.BOOLEAN : 0xFF]);
    }

    if (typeof value === 'number') {
      if (Number.isInteger(value)) {
        // INT32
        const buf = Buffer.allocUnsafe(5);
        buf[0] = BiWiTypes.INT32;
        buf.writeInt32BE(value, 1);
        return buf;
      } else {
        // FLOAT64
        const buf = Buffer.allocUnsafe(9);
        buf[0] = BiWiTypes.FLOAT64;
        buf.writeDoubleBE(value, 1);
        return buf;
      }
    }

    if (typeof value === 'string') {
      const strBuf = Buffer.from(value, 'utf8');
      const lenBuf = this._encodeVarInt(strBuf.length);
      return Buffer.concat([Buffer.from([BiWiTypes.STRING]), lenBuf, strBuf]);
    }

    if (Buffer.isBuffer(value)) {
      const lenBuf = this._encodeVarInt(value.length);
      return Buffer.concat([Buffer.from([BiWiTypes.BINARY]), lenBuf, value]);
    }

    if (Array.isArray(value)) {
      const chunks = [Buffer.from([BiWiTypes.ARRAY]), this._encodeVarInt(value.length)];
      for (const item of value) {
        chunks.push(this._encodeValue(item));
      }
      return Buffer.concat(chunks);
    }

    if (typeof value === 'object') {
      const keys = Object.keys(value);
      const chunks = [Buffer.from([BiWiTypes.OBJECT]), this._encodeVarInt(keys.length)];
      
      for (const key of keys) {
        const keyBuf = Buffer.from(key, 'utf8');
        chunks.push(this._encodeVarInt(keyBuf.length));
        chunks.push(keyBuf);
        chunks.push(this._encodeValue(value[key]));
      }
      return Buffer.concat(chunks);
    }

    throw new Error(`Unsupported type: ${typeof value}`);
  }

  /**
   * Decode a value from buffer at offset
   */
  static _decodeValue(buffer, offset) {
    const typeCode = buffer[offset];
    offset++;

    switch (typeCode) {
      case BiWiTypes.NULL:
        return { value: null, length: 1 };

      case BiWiTypes.BOOLEAN:
        return { value: true, length: 1 };

      case 0xFF:
        return { value: false, length: 1 };

      case BiWiTypes.INT32: {
        const value = buffer.readInt32BE(offset);
        return { value, length: 5 };
      }

      case BiWiTypes.FLOAT64: {
        const value = buffer.readDoubleBE(offset);
        return { value, length: 9 };
      }

      case BiWiTypes.STRING: {
        const { value: len, length: lenSize } = BiWiMessage._decodeVarInt(buffer, offset);
        offset += lenSize;
        const value = buffer.toString('utf8', offset, offset + len);
        return { value, length: 1 + lenSize + len };
      }

      case BiWiTypes.OBJECT: {
        const { value: count, length: countLen } = BiWiMessage._decodeVarInt(buffer, offset);
        offset += countLen;
        let totalLen = 1 + countLen;

        const obj = {};
        for (let i = 0; i < count; i++) {
          // Decode key
          const { value: keyLen, length: keyLenSize } = BiWiMessage._decodeVarInt(buffer, offset);
          offset += keyLenSize;
          totalLen += keyLenSize;

          const key = buffer.toString('utf8', offset, offset + keyLen);
          offset += keyLen;
          totalLen += keyLen;

          // Decode value
          const { value, length: valueLen } = BiWiMessage._decodeValue(buffer, offset);
          offset += valueLen;
          totalLen += valueLen;

          obj[key] = value;
        }

        return { value: obj, length: totalLen };
      }

      default:
        throw new Error(`Unknown type code: 0x${typeCode.toString(16)}`);
    }
  }
}

/**
 * Call Rust BiWi encoder (if needed for performance-critical operations)
 */
export async function encodeWithRust(message) {
  // For now, use JS implementation
  // Future: could spawn Rust process or use N-API addon
  return message.toBuffer();
}

/**
 * Call Rust BiWi decoder (if needed for performance-critical operations)
 */
export async function decodeWithRust(buffer) {
  // For now, use JS implementation
  // Future: could spawn Rust process or use N-API addon
  return BiWiMessage.fromBuffer(buffer);
}

/**
 * Verify Rust BiWi is available
 */
export function checkRustBiWi() {
  const libPath = join(RUST_BIN_PATH, 'libbiwi.so');
  return fs.existsSync(libPath) || fs.existsSync(join(RUST_BIN_PATH, 'libbiwi.dylib'));
}

export default {
  BiWiMessage,
  BiWiTypes,
  encodeWithRust,
  decodeWithRust,
  checkRustBiWi,
};
