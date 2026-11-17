"use client";

import { request } from "./http";

export interface AccessTeam {
  id: string;
  name: string;
  slug: string;
  createdAt: string;
  updatedAt: string;
}

export interface TeamMemberRecord {
  teamId: string;
  userId: string;
  role: string;
  createdAt: string;
}

export function listAccessTeams(): Promise<AccessTeam[]> {
  return request<AccessTeam[]>("/api/v1/teams");
}

export function createAccessTeam(body: { name: string; slug: string }): Promise<AccessTeam> {
  return request<AccessTeam>("/api/v1/teams", { method: "POST", body: JSON.stringify(body) });
}

export function deleteAccessTeam(id: string): Promise<void> {
  return request<void>(`/api/v1/teams/${id}`, { method: "DELETE" });
}

export function listTeamMembers(teamId: string): Promise<TeamMemberRecord[]> {
  return request<TeamMemberRecord[]>(`/api/v1/teams/${teamId}/members`);
}

export function addTeamMember(
  teamId: string,
  body: { userId: string; role: string },
): Promise<TeamMemberRecord> {
  return request<TeamMemberRecord>(`/api/v1/teams/${teamId}/members`, {
    method: "POST",
    body: JSON.stringify(body),
  });
}

export function removeTeamMember(teamId: string, userId: string): Promise<void> {
  return request<void>(`/api/v1/teams/${teamId}/members/${userId}`, { method: "DELETE" });
}
