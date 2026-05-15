import type { PermissionScope } from "./types.js";

export const permission = {
  readOnly(resources: string[] = ["workspace"]): PermissionScope {
    return {
      mode: "read",
      description: "Read project or run context",
      approval: "none",
      resources,
    };
  },
  writeWorkspace(options: { paths: string[]; approval?: "none" | "required" }): PermissionScope {
    return {
      mode: "write",
      description: "Write files in the workspace",
      approval: options.approval ?? "required",
      resources: options.paths,
    };
  },
  network(options: { hosts: string[]; approval?: "none" | "required" }): PermissionScope {
    return {
      mode: "network",
      description: "Call external network resources",
      approval: options.approval ?? "required",
      resources: options.hosts,
    };
  },
  process(options: { commands: string[]; approval?: "none" | "required" }): PermissionScope {
    return {
      mode: "process",
      description: "Run local commands or background processes",
      approval: options.approval ?? "required",
      resources: options.commands,
    };
  },
};

export function requiresApproval(scopes: PermissionScope[]): PermissionScope | undefined {
  return scopes.find((scope) => scope.approval === "required");
}
