<template>
  <el-form
    ref="formRef"
    :model="form"
    :rules="rules"
    label-position="top"
    @submit.prevent="handleSubmit"
  >
    <el-form-item label="用户名" prop="username">
      <el-input
        v-model="form.username"
        placeholder="请输入用户名"
        :prefix-icon="User"
        autocomplete="username"
      />
    </el-form-item>
    <el-form-item label="密码" prop="password">
      <el-input
        v-model="form.password"
        type="password"
        placeholder="请输入密码"
        :prefix-icon="Lock"
        show-password
        autocomplete="current-password"
        @keyup.enter="handleSubmit"
      />
    </el-form-item>
    <el-form-item>
      <el-button
        type="primary"
        :loading="loading"
        style="width: 100%"
        native-type="submit"
      >
        登录
      </el-button>
    </el-form-item>
  </el-form>
</template>

<script setup lang="ts">
import { reactive, ref } from "vue";
import { User, Lock } from "@element-plus/icons-vue";
import type { FormInstance, FormRules } from "element-plus";

const emit = defineEmits<{
  submit: [username: string, password: string];
}>();

defineProps<{
  loading: boolean;
}>();

const formRef = ref<FormInstance>();
const form = reactive({
  username: "",
  password: "",
});

const rules: FormRules = {
  username: [{ required: true, message: "请输入用户名", trigger: "blur" }],
  password: [{ required: true, message: "请输入密码", trigger: "blur" }],
};

async function handleSubmit() {
  if (!formRef.value) return;
  const valid = await formRef.value.validate().catch(() => false);
  if (!valid) return;
  emit("submit", form.username, form.password);
}
</script>
