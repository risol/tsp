/**
 * Crypto E2E Test Page - TSP File
 * Test Crypto full functionality (native Deno Web Crypto API)
 */

export default Page(async function(ctx, { crypto, response }) {
  const results: Record<string, unknown> = {};

  // Test 1: getRandomValues
  try {
    const randomBytes = crypto.getRandomValues(16);
    results.test1_random_values = {
      status: randomBytes.length === 16 ? "✓ PASSED" : "✗ FAILED",
      data: {
        length: randomBytes.length,
        isUint8Array: randomBytes instanceof Uint8Array,
      },
    };
  } catch (error) {
    results.test1_random_values = {
      status: "✗ FAILED",
      error: (error as Error).message,
    };
  }

  // Test 2: digest (SHA-256)
  try {
    const hashBuffer = await crypto.digest("SHA-256", "Hello World");
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    const hashHex = hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");

    results.test2_sha256_hash = {
      status: hashHex.length === 64 ? "✓ PASSED" : "✗ FAILED",
      data: {
        hashLength: hashHex.length,
        hashHex: hashHex.substring(0, 16) + "...",
      },
    };
  } catch (error) {
    results.test2_sha256_hash = {
      status: "✗ FAILED",
      error: (error as Error).message,
    };
  }

  // Test 3: generateKey (AES-GCM)
  try {
    const key = await crypto.generateKey("AES-GCM", 256);
    // Test encryption to verify key works
    const iv = crypto.getRandomValues(12);
    const testData = new TextEncoder().encode("test");
    const encrypted = await crypto.encrypt(testData, key, iv);
    const decrypted = await crypto.decrypt(encrypted, key, iv);
    const decryptedText = new TextDecoder().decode(decrypted);

    results.test3_aes_key_generation = {
      status: decryptedText === "test" ? "✓ PASSED" : "✗ FAILED",
      data: {
        encryptionWorks: decryptedText === "test",
      },
    };
  } catch (error) {
    results.test3_aes_key_generation = {
      status: "✗ FAILED",
      error: (error as Error).message,
    };
  }

  // Test 4: AES-GCM encrypt/decrypt
  try {
    const key = await crypto.generateKey("AES-GCM", 256);
    const iv = crypto.getRandomValues(12);
    const plaintext = new TextEncoder().encode("Secret message");

    const encrypted = await crypto.encrypt(plaintext, key, iv);
    const decrypted = await crypto.decrypt(encrypted, key, iv);
    const decryptedText = new TextDecoder().decode(decrypted);

    results.test4_aes_encrypt_decrypt = {
      status: decryptedText === "Secret message" ? "✓ PASSED" : "✗ FAILED",
      data: {
        original: "Secret message",
        decrypted: decryptedText,
      },
    };
  } catch (error) {
    results.test4_aes_encrypt_decrypt = {
      status: "✗ FAILED",
      error: (error as Error).message,
    };
  }

  // Test 5: HMAC sign/verify
  try {
    const key = await crypto.generateKey("HMAC");
    const data = new TextEncoder().encode("Message to sign");

    const signature = await crypto.sign("HMAC", key, data);
    const isValid = await crypto.verify("HMAC", key, signature, data);

    results.test5_hmac_sign_verify = {
      status: isValid === true ? "✓ PASSED" : "✗ FAILED",
      data: {
        signatureLength: signature.byteLength,
        isValid,
      },
    };
  } catch (error) {
    results.test5_hmac_sign_verify = {
      status: "✗ FAILED",
      error: (error as Error).message,
    };
  }

  // Test 6: importKey
  try {
    // AES-256 requires exactly 32 bytes key
    const keyData = new TextEncoder().encode("12345678901234567890123456789012"); // 32 bytes
    const key = await crypto.importKey("AES-GCM", keyData);

    // Test if key works by encrypting
    const iv = crypto.getRandomValues(12);
    const testData = new TextEncoder().encode("test");
    const encrypted = await crypto.encrypt(testData, key, iv);

    results.test6_import_key = {
      status: encrypted.byteLength > 0 ? "✓ PASSED" : "✗ FAILED",
      data: {
        algorithm: key.algorithm.name,
        encryptedLength: encrypted.byteLength,
      },
    };
  } catch (error) {
    results.test6_import_key = {
      status: "✗ FAILED",
      error: (error as Error).message,
    };
  }

  // Test 7: digest with Uint8Array
  try {
    const data = new Uint8Array([72, 101, 108, 108, 111]); // "Hello"
    const hashBuffer = await crypto.digest("SHA-256", data);

    results.test7_digest_uint8array = {
      status: hashBuffer.byteLength === 32 ? "✓ PASSED" : "✗ FAILED",
      data: {
        hashLength: hashBuffer.byteLength,
      },
    };
  } catch (error) {
    results.test7_digest_uint8array = {
      status: "✗ FAILED",
      error: (error as Error).message,
    };
  }

  // Test 8: SHA-1 hash
  try {
    const hashBuffer = await crypto.digest("SHA-1", "Test");

    results.test8_sha1_hash = {
      status: hashBuffer.byteLength === 20 ? "✓ PASSED" : "✗ FAILED",
      data: {
        hashLength: hashBuffer.byteLength,
      },
    };
  } catch (error) {
    results.test8_sha1_hash = {
      status: "✗ FAILED",
      error: (error as Error).message,
    };
  }

  // Test 9: Different IV produces different ciphertext
  try {
    const key = await crypto.generateKey("AES-GCM", 256);
    const data = new TextEncoder().encode("Same data");

    const iv1 = crypto.getRandomValues(12);
    const iv2 = crypto.getRandomValues(12);

    const encrypted1 = await crypto.encrypt(data, key, iv1);
    const encrypted2 = await crypto.encrypt(data, key, iv2);

    // Convert to string for comparison
    const arr1 = Array.from(new Uint8Array(encrypted1));
    const arr2 = Array.from(new Uint8Array(encrypted2));
    const isDifferent = JSON.stringify(arr1) !== JSON.stringify(arr2);

    results.test9_different_iv = {
      status: isDifferent ? "✓ PASSED" : "✗ FAILED",
      data: {
        differentCiphertext: isDifferent,
      },
    };
  } catch (error) {
    results.test9_different_iv = {
      status: "✗ FAILED",
      error: (error as Error).message,
    };
  }

  // Test 10: AES-128 key
  try {
    const key = await crypto.generateKey("AES-GCM", 128);
    // Test encryption to verify key works
    const iv = crypto.getRandomValues(12);
    const testData = new TextEncoder().encode("test128");
    const encrypted = await crypto.encrypt(testData, key, iv);
    const decrypted = await crypto.decrypt(encrypted, key, iv);
    const decryptedText = new TextDecoder().decode(decrypted);

    results.test10_aes128_key = {
      status: decryptedText === "test128" ? "✓ PASSED" : "✗ FAILED",
      data: {
        encryptionWorks: decryptedText === "test128",
      },
    };
  } catch (error) {
    results.test10_aes128_key = {
      status: "✗ FAILED",
      error: (error as Error).message,
    };
  }

  // Calculate test results
  const passed = Object.values(results).filter((r: any) => r.status === "✓ PASSED").length;
  const total = Object.keys(results).length;

  return response.json({
    testSuite: "Crypto E2E Tests",
    total,
    passed,
    failed: total - passed,
    results,
  });
});
