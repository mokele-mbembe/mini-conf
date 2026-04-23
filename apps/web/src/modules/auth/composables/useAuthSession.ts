import { ref, computed } from "vue";
import { defineStore } from "pinia";
import type { AuthUser } from "@/api/types/auth";
import * as authApi from "@/api/auth";
import { isApiError } from "@/api/error";
import { t } from "@/shared/i18n";

export type SessionCheckResult = "authenticated" | "unauthenticated" | "error";

export const useAuthSession = defineStore("authSession", () => {
  const user = ref<AuthUser | null>(null);
  const checked = ref(false);
  const sessionError = ref<string | null>(null);

  const isLoggedIn = () => user.value !== null;

  const isPlatformAdmin = computed(
    () => user.value?.is_platform_admin ?? false,
  );
  const mustChangePassword = computed(
    () => user.value?.must_change_password ?? false,
  );
  const userStatus = computed(() => user.value?.status ?? null);

  async function checkSession(): Promise<SessionCheckResult> {
    sessionError.value = null;
    try {
      const res = await authApi.getMe();
      user.value = res.user;
      checked.value = true;
      return "authenticated";
    } catch (err) {
      checked.value = true;
      if (isApiError(err) && err.status === 401) {
        user.value = null;
        return "unauthenticated";
      }
      // Network error or 5xx — do NOT clear user, do NOT treat as logged-out
      sessionError.value = t("login.sessionCheckFailed");
      return "error";
    }
  }

  async function login(username: string, password: string): Promise<void> {
    const res = await authApi.login({ username, password });
    user.value = res.user;
    checked.value = true;
  }

  async function changePassword(
    currentPassword: string,
    newPassword: string,
  ): Promise<void> {
    const res = await authApi.changePassword({
      current_password: currentPassword,
      new_password: newPassword,
    });
    user.value = res.user;
    checked.value = true;
  }

  async function logout(): Promise<void> {
    try {
      await authApi.logout();
    } finally {
      user.value = null;
    }
  }

  return {
    user,
    checked,
    sessionError,
    isLoggedIn,
    isPlatformAdmin,
    mustChangePassword,
    userStatus,
    checkSession,
    login,
    changePassword,
    logout,
  };
});
