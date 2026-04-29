<template>
  <div />
</template>

<script setup lang="ts">
import { onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ROUTE_NAMES } from "@/shared/constants/routes";

const route = useRoute();
const router = useRouter();

onMounted(() => {
  void router.replace({
    name: ROUTE_NAMES.DEPLOYMENT_LIST,
    params: {
      projectId: route.params.projectId,
    },
    query: {
      ...route.query,
      expandedDeploymentId: String(route.params.deploymentId),
    },
  });
});
</script>
