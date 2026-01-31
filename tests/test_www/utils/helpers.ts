/**
 * 工具函数 - 测试导入非 src 目录下的 TS 文件
 */

/**
 * 格式化日期
 */
export function formatDate(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

/**
 * 生成问候语
 */
export function getGreeting(name?: string): string {
  const hour = new Date().getHours();
  let timeGreeting = "早上好";

  if (hour >= 12 && hour < 18) {
    timeGreeting = "下午好";
  } else if (hour >= 18) {
    timeGreeting = "晚上好";
  }

  return name ? `${timeGreeting}，${name}！` : `${timeGreeting}！`;
}

/**
 * 计算数组总和
 */
export function sum(numbers: number[]): number {
  return numbers.reduce((acc, num) => acc + num, 0);
}

/**
 * 截断文本
 */
export function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) {
    return text;
  }
  return text.substring(0, maxLength) + "...";
}
