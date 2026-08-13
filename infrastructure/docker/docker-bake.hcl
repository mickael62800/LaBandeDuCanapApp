# ============================================================================
# BuildKit Bake — construit en parallele les images reellement deployees par
# docker-compose.yml. Lancer depuis la racine du depot :
#
#   docker buildx bake -f infrastructure/docker/docker-bake.hcl
#   docker buildx bake -f infrastructure/docker/docker-bake.hcl core
#   docker buildx bake -f infrastructure/docker/docker-bake.hcl workers
#   docker buildx bake -f infrastructure/docker/docker-bake.hcl atrium nexus
# ============================================================================

variable "TAG" { default = "latest" }

group "default" {
  targets = [
    "platform-api", "web", "auth-api", "docker-agent", "ops-agent",
    "platform-scheduler",
    "platform-gateway", "sentinel-bot",
    "atrium-bot", "nexus-bot",
  ]
}

group "core" {
  targets = [
    "platform-api", "web", "auth-api", "docker-agent", "ops-agent",
    "platform-scheduler",
    "platform-gateway", "sentinel-bot",
  ]
}

group "workers" {
  targets = ["platform-scheduler", "ops-agent"]
}

group "atrium" {
  targets = ["platform-api", "atrium-bot"]
}

group "nexus" {
  targets = ["platform-api", "nexus-bot"]
}

target "_alpine-base" {
  context    = "."
  dockerfile = "infrastructure/docker/Dockerfile.rust-alpine"
}

target "_debian-base" {
  context    = "."
  dockerfile = "infrastructure/docker/Dockerfile.rust-debian"
}

target "platform-gateway" {
  inherits = ["_alpine-base"]
  args     = { BIN_NAME = "platform-gateway" }
  tags     = ["discordsentinel-platform-gateway:${TAG}"]
}

target "platform-api" {
  inherits = ["_debian-base"]
  args = {
    BIN_NAME       = "platform-api"
    MIGRATIONS_SRC = "platform-api/migrations"
  }
  tags = ["discordsentinel-platform-api:${TAG}"]
}

# Toutes les autres applications Rust partagent le meme Dockerfile Alpine.
# La matrice maintient un seul cache Cargo Chef/BuildKit par dependances tout
# en produisant une cible et une image distinctes par binaire.
target "rust-app" {
  inherits = ["_alpine-base"]
  matrix = {
    app = [
      "auth-api", "docker-agent", "ops-agent", "platform-scheduler",
      "sentinel-bot",
      "atrium-bot", "nexus-bot",
    ]
  }
  name = "${app}"
  args = { BIN_NAME = "${app}" }
  tags = ["discordsentinel-${app}:${TAG}"]
}

target "web" {
  context    = "."
  dockerfile = "web/Dockerfile"
  tags       = ["discordsentinel-web:${TAG}"]
}
