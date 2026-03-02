/**
 * Type Definition File (.ts)
 * Used to test type imports
 */

export enum UserRole {
  Admin = "admin",
  Developer = "developer",
  User = "user",
}

export interface User {
  id: number;
  name: string;
  role: UserRole;
  email: string;
  createdAt?: Date;
}

export interface Permission {
  resource: string;
  actions: string[];
}

export interface RolePermissions {
  role: UserRole;
  permissions: Permission[];
}

export const ADMIN_PERMISSIONS: Permission[] = [
  { resource: "users", actions: ["read", "write", "delete"] },
  { resource: "settings", actions: ["read", "write"] },
];

export const DEVELOPER_PERMISSIONS: Permission[] = [
  { resource: "users", actions: ["read", "write"] },
  { resource: "settings", actions: ["read"] },
];

export const USER_PERMISSIONS: Permission[] = [
  { resource: "profile", actions: ["read", "write"] },
];
