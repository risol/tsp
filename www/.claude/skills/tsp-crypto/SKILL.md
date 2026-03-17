---
name: tsp-crypto
description: Cryptography utilities in TSP. Use when hashing passwords, encrypting data, or using bcrypt for authentication.
---

# TSP Cryptography

Use this skill for cryptography operations in TSP.

## crypto - Built-in Crypto

```typescript
export default Page(async function(ctx, { crypto, response }) {
  // Generate random values
  const randomBytes = crypto.getRandomValues(16);
  // Returns: Uint8Array(16)

  // Hash (SHA-256)
  const hash = await crypto.digest('SHA-256', 'Hello World');
  // Returns: ArrayBuffer

  // Hash (other algorithms)
  const sha1 = await crypto.digest('SHA-1', 'data');
  const sha512 = await crypto.digest('SHA-512', 'data');
  const md5 = await crypto.digest('MD5', 'data');  // Note: MD5 is broken

  // Convert to hex string
  const hashHex = Array.from(new Uint8Array(hash))
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');

  // Generate AES-GCM key
  const key = await crypto.generateKey('AES-GCM', 256);
  // Returns: CryptoKey

  // Generate HMAC key
  const hmacKey = await crypto.generateKey('HMAC', 256);

  // Encrypt with AES-GCM
  const iv = crypto.getRandomValues(12);
  const plaintext = new TextEncoder().encode('secret message');
  const encrypted = await crypto.encrypt(plaintext, key, iv);

  // Decrypt
  const decrypted = await crypto.decrypt(encrypted, key, iv);
  const decryptedText = new TextDecoder().decode(decrypted);

  // Sign with HMAC
  const signature = await crypto.sign(hmacKey, plaintext);

  // Verify HMAC
  const isValid = await crypto.verify(hmacKey, signature, plaintext);

  return response.json({
    hash: hashHex,
    decryptedText,
    isValid
  });
});
```

## createBcryptjs - Password Hashing

```typescript
export default Page(async function(ctx, { createBcryptjs, response }) {
  const bcrypt = await createBcryptjs({ saltRounds: 10 });

  // Hash password
  const hash = await bcrypt.hash('password123');
  // Returns: $2b$10$...

  // Verify password
  const isValid = await bcrypt.compare('password123', hash);
  // Returns: true

  const isInvalid = await bcrypt.compare('wrong', hash);
  // Returns: false

  return response.json({
    hash,
    isValid,
    isInvalid: !isInvalid
  });
});
```

## Algorithms Reference

### Hash Algorithms
| Algorithm | Use Case |
|-----------|----------|
| SHA-256 | General purpose (recommended) |
| SHA-512 | Higher security |
| MD5 | Checksums only (not for passwords) |

### Encryption
| Algorithm | Key Sizes | Use Case |
|-----------|-----------|----------|
| AES-GCM | 128, 256 | Authenticated encryption |
| HMAC | Any | Message signing |

### bcrypt
| Salt Rounds | Time | Security |
|-------------|------|----------|
| 8 | ~100ms | Minimum |
| 10 | ~300ms | Recommended |
| 12 | ~1s | High security |

## Best Practices

- Use bcrypt for password hashing - never use raw SHA-256 for passwords
- Use 10 salt rounds for bcrypt (balance of security and performance)
- Use AES-GCM for encryption (includes authentication)
- Always use unique IV for encryption
- Store IV alongside ciphertext
- Use HMAC for message authentication, not for encryption
