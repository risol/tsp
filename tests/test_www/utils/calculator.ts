/**
 * Calculator utility functions (.ts)
 * Test importing nested .ts files
 */

export function calculateTax(amount: number, rate = 0.1): number {
  return Math.round(amount * rate * 100) / 100;
}

export function calculateDiscount(amount: number, percentage: number): number {
  return Math.round(amount * (percentage / 100) * 100) / 100;
}

export function formatCurrency(amount: number, currency = "USD"): string {
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency,
  }).format(amount);
}

export function sum(...numbers: number[]): number {
  return numbers.reduce((acc, n) => acc + n, 0);
}

export function multiply(...numbers: number[]): number {
  return numbers.reduce((acc, n) => acc * n, 1);
}
