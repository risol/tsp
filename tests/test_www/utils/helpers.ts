/**
 * Utility functions - Test importing TS files outside src directory
 */

/**
 * Format date
 */
export function formatDate(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

/**
 * Generate greeting
 */
export function getGreeting(name?: string): string {
  const hour = new Date().getHours();
  let timeGreeting = "Good morning";

  if (hour >= 12 && hour < 18) {
    timeGreeting = "Good afternoon";
  } else if (hour >= 18) {
    timeGreeting = "Good evening";
  }

  return name ? `${timeGreeting}, ${name}!` : `${timeGreeting}!`;
}

/**
 * Calculate array sum
 */
export function sum(numbers: number[]): number {
  return numbers.reduce((acc, num) => acc + num, 0);
}

/**
 * Truncate text
 */
export function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) {
    return text;
  }
  return text.substring(0, maxLength) + "...";
}
