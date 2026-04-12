import type { Router } from "vue-router";
import { useAuthSession } from "@/modules/auth/composables/useAuthSession";
import { ROUTE_NAMES } from "@/shared/constants/routes";

export function setupGuards(router: Router) {
  router.beforeEach(async (to) => {
    const authSession = useAuthSession();

    // If the session hasn't been checked yet, do so now
    if (!authSession.checked) {
      const result = await authSession.checkSession();
      // On system error, let the user land on /login where the error is shown,
      // but do NOT force-redirect away from login as if authenticated.
      if (result === "error") {
        if (to.name !== ROUTE_NAMES.LOGIN) {
          return { name: ROUTE_NAMES.LOGIN };
        }
        return true;
      }
    }

    const isLoggedIn = authSession.isLoggedIn();
    const requiresAuth = to.matched.some(
      (record) => record.meta.requiresAuth !== false,
    );

    // Logged-in user visiting /login -> redirect to projects
    if (isLoggedIn && to.name === ROUTE_NAMES.LOGIN) {
      return { name: ROUTE_NAMES.PROJECTS };
    }

    // Not logged in visiting protected route -> redirect to login
    if (!isLoggedIn && requiresAuth) {
      return { name: ROUTE_NAMES.LOGIN };
    }

    return true;
  });
}
