/**
 * Test Config File (.ts)
 * Used to test .ts file import path handling
 */

export interface AppConfig {
  name: string;
  version: string;
  isProduction: boolean;
  port: number;
  features: string[];
}

export const appConfig: AppConfig = {
  name: "TSP Test App",
  version: "1.0.0",
  isProduction: false,
  port: 9001,
  features: ["hot-reload", "typescript", "jsx", "e2e-tests"],
};

export function getConfig(): AppConfig {
  return appConfig;
}

export function getFeatureList(): string[] {
  return appConfig.features;
}
