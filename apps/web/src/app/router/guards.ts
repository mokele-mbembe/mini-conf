import type { Router } from "vue-router";
import { useAuthSession } from "@/modules/auth/composables/useAuthSession";
import { useSetupStatus } from "@/modules/setup/composables/useSetupStatus";
import { ROUTE_NAMES } from "@/shared/constants/routes";

export function setupGuards(router: Router) {
  router.beforeEach(async (to) => {
    const authSession = useAuthSession();
    const setupStatus = useSetupStatus();

    if (!setupStatus.checked) {
      await setupStatus.checkStatus();
    }

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
    const requiresPlatformAdmin = to.matched.some(
      (record) => record.meta.requiresPlatformAdmin === true,
    );

    if (
      to.name === ROUTE_NAMES.SETUP &&
      setupStatus.checked &&
      !setupStatus.setupRequired
    ) {
      return isLoggedIn
        ? { name: ROUTE_NAMES.PROJECTS }
        : { name: ROUTE_NAMES.LOGIN };
    }

    if (
      setupStatus.setupRequired &&
      isLoggedIn &&
      to.name !== ROUTE_NAMES.SETUP
    ) {
      return { name: ROUTE_NAMES.SETUP };
    }

    // Logged-in user visiting /login -> redirect based on role
    if (isLoggedIn && to.name === ROUTE_NAMES.LOGIN) {
      const nextRoute = setupStatus.setupRequired
        ? { name: ROUTE_NAMES.SETUP }
        : authSession.isPlatformAdmin
          ? { name: ROUTE_NAMES.ADMIN_DASHBOARD }
          : { name: ROUTE_NAMES.PROJECTS };
      return nextRoute;
    }

    // Not logged in visiting protected route -> redirect to login
    if (!isLoggedIn && requiresAuth) {
      return { name: ROUTE_NAMES.LOGIN };
    }

    // Requires platform admin but user is not platform admin -> go to projects
    if (isLoggedIn && requiresPlatformAdmin && !authSession.isPlatformAdmin) {
      return { name: ROUTE_NAMES.PROJECTS };
    }

    return true;
  });
}
