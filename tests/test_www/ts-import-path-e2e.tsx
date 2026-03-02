/**
 * .ts file import path E2E test
 * Test scenario: Preprocessor correctly handles .ts file import paths
 *
 * Problem background:
 * - Import in source file: import { config } from './config'
 * - After precompilation should be: import { config } from './config.ts'
 * - If .ts extension is missing, compiled binary cannot find module
 *
 * Test content:
 * 1. Import .ts files without extension
 * 2. Import .ts files with extension
 * 3. Import nested .ts files
 * 4. Verify compiled file path correctness
 */

import { appConfig, AppConfig } from "./config";
import { calculateTax, formatCurrency } from "./utils/calculator.ts";
import { User, UserRole } from "./types";

export default Page(async function(ctx, { response }) {
  // Note: This test page does not need to read request body
  // All test data comes from imported .ts files

  // Test 1: .ts import without extension
  const appVersion = appConfig.version;
  const isProduction = appConfig.isProduction;

  // Test 2: .ts import with extension
  const price = 100;
  const tax = calculateTax(price);
  const totalPrice = price + tax;
  const formattedPrice = formatCurrency(totalPrice);

  // Test 3: Type imports
  const admin: User = {
    id: 1,
    name: "Admin",
    role: UserRole.Admin,
    email: "admin@example.com",
  };

  const developer: User = {
    id: 2,
    name: "Developer",
    role: UserRole.Developer,
    email: "dev@example.com",
  };

  // Test 4: Nested .ts file imports
  const allRoles: UserRole[] = [UserRole.Admin, UserRole.Developer, UserRole.User];

  // Validation results
  const results = {
    // Basic import test
    basic: {
      appName: appConfig.name,
      appVersion,
      isProduction,
      passed: appConfig.name === "TSP Test App" && appVersion === "1.0.0",
    },

    // Function import test
    functions: {
      price,
      tax,
      totalPrice,
      formattedPrice,
      passed: tax === 10 && totalPrice === 110,
    },

    // Type import test
    types: {
      adminName: admin.name,
      adminRole: admin.role,
      developerName: developer.name,
      developerRole: developer.role,
      passed: admin.role === UserRole.Admin && developer.role === UserRole.Developer,
    },

    // Nested import test
    nested: {
      roleCount: allRoles.length,
      passed: allRoles.length === 3,
    },

    // Summary
    allPassed: false,
  };

  // Calculate overall test result
  results.allPassed =
    results.basic.passed &&
    results.functions.passed &&
    results.types.passed &&
    results.nested.passed;

  // Return JSON response (for E2E automated testing)
  return response.json(results);
});
