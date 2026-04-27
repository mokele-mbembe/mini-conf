# Runbooks

这组文档记录生产和准生产环境的部署、初始化、运维与恢复流程。

当前生产交付主路径：

- [PRODUCTION_BINARY.md](./PRODUCTION_BINARY.md)

发布包由仓库内命令生成：

```bash
just release-package
```

也可以通过 GitHub Actions 的 `Release Package` workflow 手动生成，或推送 `v*` tag 自动生成。

当前不把 `docker-compose.yml` 作为 MVP 生产交付目标。Docker image 可以作为后续可选包装形式，但不是默认运行模型。
