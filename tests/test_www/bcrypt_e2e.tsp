/**
 * Bcryptjs E2E Test Page - TSP File
 * Test Bcryptjs password hashing functionality
 */

export default Page(async function(ctx, { createBcryptjs, response }) {
  const results: Record<string, unknown> = {};

  // Try to initialize bcryptjs
  let bcrypt = null;
  let bcryptError = null;
  try {
    bcrypt = await createBcryptjs({ saltRounds: 10 });
  } catch (error) {
    bcryptError = (error as Error).message;
  }

  // Test 1: Basic hash and verify
  if (!bcrypt) {
    results.test1_basic_hash_verify = {
      status: "✗ FAILED",
      error: bcryptError || "Bcryptjs not available",
    };
  } else {
    try {
      const password = "mySecretPassword123";
      const hash = bcrypt.hash(password);

      results.test1_basic_hash_verify = {
        status: typeof hash === "string" && hash.length > 0 ? "✓ PASSED" : "✗ FAILED",
        data: {
          hashLength: hash.length,
          hashPrefix: hash.substring(0, 7),
        },
      };
    } catch (error) {
      results.test1_basic_hash_verify = {
        status: "✗ FAILED",
        error: (error as Error).message,
      };
    }
  }

  // Test 2: Verify correct password
  if (!bcrypt) {
    results.test2_verify_correct_password = {
      status: "✗ FAILED",
      error: "Bcryptjs not available",
    };
  } else {
    try {
      const password = "correctPassword";
      const hash = bcrypt.hash(password);
      const isValid = bcrypt.compare(password, hash);

      results.test2_verify_correct_password = {
        status: isValid === true ? "✓ PASSED" : "✗ FAILED",
        data: { isValid },
      };
    } catch (error) {
      results.test2_verify_correct_password = {
        status: "✗ FAILED",
        error: (error as Error).message,
      };
    }
  }

  // Test 3: Verify wrong password
  if (!bcrypt) {
    results.test3_verify_wrong_password = {
      status: "✗ FAILED",
      error: "Bcryptjs not available",
    };
  } else {
    try {
      const password = "correctPassword";
      const wrongPassword = "wrongPassword";
      const hash = bcrypt.hash(password);
      const isValid = bcrypt.compare(wrongPassword, hash);

      results.test3_verify_wrong_password = {
        status: isValid === false ? "✓ PASSED" : "✗ FAILED",
        data: { isValid },
      };
    } catch (error) {
      results.test3_verify_wrong_password = {
        status: "✗ FAILED",
        error: (error as Error).message,
      };
    }
  }

  // Test 4: Different salt generates different hash
  if (!bcrypt) {
    results.test4_different_salt = {
      status: "✗ FAILED",
      error: "Bcryptjs not available",
    };
  } else {
    try {
      const password = "samePassword";
      const hash1 = bcrypt.hash(password);
      const hash2 = bcrypt.hash(password);

      results.test4_different_salt = {
        status: hash1 !== hash2 ? "✓ PASSED" : "✗ FAILED",
        data: {
          hash1: hash1.substring(0, 10),
          hash2: hash2.substring(0, 10),
          areDifferent: hash1 !== hash2,
        },
      };
    } catch (error) {
      results.test4_different_salt = {
        status: "✗ FAILED",
        error: (error as Error).message,
      };
    }
  }

  // Test 5: Same password still validates with stored hash
  if (!bcrypt) {
    results.test5_consistent_validation = {
      status: "✗ FAILED",
      error: "Bcryptjs not available",
    };
  } else {
    try {
      const password = "consistentPassword";
      const storedHash = bcrypt.hash(password);
      const bcrypt2 = await createBcryptjs();
      const isValid = bcrypt2.compare(password, storedHash);

      results.test5_consistent_validation = {
        status: isValid === true ? "✓ PASSED" : "✗ FAILED",
        data: { isValid },
      };
    } catch (error) {
      results.test5_consistent_validation = {
        status: "✗ FAILED",
        error: (error as Error).message,
      };
    }
  }

  // Test 6: Custom salt rounds
  if (!bcrypt) {
    results.test6_custom_salt_rounds = {
      status: "✗ FAILED",
      error: "Bcryptjs not available",
    };
  } else {
    try {
      const bcryptFast = await createBcryptjs({ saltRounds: 4 });
      const bcryptSlow = await createBcryptjs({ saltRounds: 12 });
      const password = "testPassword";
      const hashFast = bcryptFast.hash(password);
      const hashSlow = bcryptSlow.hash(password);

      results.test6_custom_salt_rounds = {
        status: typeof hashFast === "string" && typeof hashSlow === "string" ? "✓ PASSED" : "✗ FAILED",
        data: {
          fastHashLength: hashFast.length,
          slowHashLength: hashSlow.length,
        },
      };
    } catch (error) {
      results.test6_custom_salt_rounds = {
        status: "✗ FAILED",
        error: (error as Error).message,
      };
    }
  }

  // Test 7: Default salt rounds (10)
  if (!bcrypt) {
    results.test7_default_salt_rounds = {
      status: "✗ FAILED",
      error: "Bcryptjs not available",
    };
  } else {
    try {
      const password = "defaultRounds";
      const hash = bcrypt.hash(password);

      results.test7_default_salt_rounds = {
        status: typeof hash === "string" ? "✓ PASSED" : "✗ FAILED",
        data: {
          hashPrefix: hash.substring(0, 7),
        },
      };
    } catch (error) {
      results.test7_default_salt_rounds = {
        status: "✗ FAILED",
        error: (error as Error).message,
      };
    }
  }

  // Test 8: Empty password
  if (!bcrypt) {
    results.test8_empty_password = {
      status: "✗ FAILED",
      error: "Bcryptjs not available",
    };
  } else {
    try {
      const password = "";
      const hash = bcrypt.hash(password);
      const isValid = bcrypt.compare(password, hash);

      results.test8_empty_password = {
        status: isValid === true ? "✓ PASSED" : "✗ FAILED",
        data: {
          hashLength: hash.length,
          isValid,
        },
      };
    } catch (error) {
      results.test8_empty_password = {
        status: "✗ FAILED",
        error: (error as Error).message,
      };
    }
  }

  // Test 9: Long password
  if (!bcrypt) {
    results.test9_long_password = {
      status: "✗ FAILED",
      error: "Bcryptjs not available",
    };
  } else {
    try {
      const password = "a".repeat(1000);
      const hash = bcrypt.hash(password);
      const isValid = bcrypt.compare(password, hash);

      results.test9_long_password = {
        status: isValid === true ? "✓ PASSED" : "✗ FAILED",
        data: {
          passwordLength: password.length,
          hashLength: hash.length,
          isValid,
        },
      };
    } catch (error) {
      results.test9_long_password = {
        status: "✗ FAILED",
        error: (error as Error).message,
      };
    }
  }

  // Test 10: Special characters in password
  if (!bcrypt) {
    results.test10_special_characters = {
      status: "✗ FAILED",
      error: "Bcryptjs not available",
    };
  } else {
    try {
      const password = "p@ssw0rd!#$%^&*()_+-=[]{}|;':\",./<>?";
      const hash = bcrypt.hash(password);
      const isValid = bcrypt.compare(password, hash);

      results.test10_special_characters = {
        status: isValid === true ? "✓ PASSED" : "✗ FAILED",
        data: { isValid },
      };
    } catch (error) {
      results.test10_special_characters = {
        status: "✗ FAILED",
        error: (error as Error).message,
      };
    }
  }

  // Calculate test results
  const passed = Object.values(results).filter((r: any) => r.status === "✓ PASSED").length;
  const total = Object.keys(results).length;

  return response.json({
    testSuite: "Bcryptjs E2E Tests",
    total,
    passed,
    failed: total - passed,
    results,
  });
});
