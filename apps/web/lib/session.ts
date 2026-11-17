"use client";

import { setAccessToken } from "./api";

/**
 * Clear the browser session and return to the login screen.
 *
 * ORBIT stores the OIDC id_token in localStorage only; there is no server-side browser
 * session to revoke. API tokens are independent and unaffected.
 */
export function signOut() {
  setAccessToken(null);
  if (typeof window !== "undefined") {
    window.location.assign("/login");
  }
}
