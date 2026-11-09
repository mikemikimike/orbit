SHELL := /bin/bash
REGISTRY ?= 192.168.1.201:5000
TAG ?= 0.1.0
NAMESPACE ?= orbit
KUBE_CONTEXT ?= microk8s
VALUES ?= deploy/helm/orbit/values.yaml
API_URL ?= http://localhost:8080

.PHONY: help dev-infra compose-up compose-down api worker web test test-db fmt clippy \
        typecheck build-web api-client helm-lint helm-template images push deploy smoke-k8s

help:
	@echo "ORBIT targets"
	@echo "  dev-infra      Start Postgres + NATS only (native development)"
	@echo "  compose-up     Full local stack via Docker Compose"
	@echo "  api/worker     Run Rust services against the dev stack"
	@echo "  web            Run Next.js (API proxy to localhost:8080)"
	@echo "  test           Rust tests (DB tests skip without ORBIT_TEST_DATABASE_URL)"
	@echo "  test-db        Rust tests including PostgreSQL integration tests"
	@echo "  fmt/clippy     Rust formatting and lint gate"
	@echo "  typecheck      TypeScript typecheck for the web app"
	@echo "  images         Build linux/amd64 images for $(REGISTRY) tag $(TAG)"
	@echo "  push           Push images to $(REGISTRY)"
	@echo "  deploy         helm upgrade orbit ($(KUBE_CONTEXT), $(NAMESPACE), $(VALUES))"
	@echo "  helm-lint      helm lint + template validation"
	@echo "  smoke-k8s      End-to-end smoke test against $(API_URL)"

dev-infra:
	docker compose -f deploy/compose/compose.dev.yaml up -d

compose-up:
	docker compose -f deploy/compose/compose.yaml up --build

compose-down:
	docker compose -f deploy/compose/compose.yaml down

api:
	cargo run -p orbit-api-server

worker:
	cargo run -p orbit-worker

web:
	pnpm --filter @orbit/web dev

test:
	cargo test --workspace

test-db:
	ORBIT_TEST_DATABASE_URL=$${ORBIT_TEST_DATABASE_URL:-postgres://orbit:orbit@localhost:5432/orbit} cargo test --workspace

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

typecheck:
	pnpm --filter @orbit/web typecheck

build-web:
	pnpm --filter @orbit/web build

api-client:
	pnpm api:generate

helm-lint:
	helm lint deploy/helm/orbit -f deploy/helm/orbit/values.yaml
	helm lint deploy/helm/orbit -f deploy/helm/orbit/values-prod.example.yaml
	helm template orbit deploy/helm/orbit -n $(NAMESPACE) \
	  -f deploy/helm/orbit/values.yaml > /tmp/orbit-rendered.yaml
	helm template orbit deploy/helm/orbit -n $(NAMESPACE) \
	  -f deploy/helm/orbit/values-prod.example.yaml > /tmp/orbit-rendered-prod.yaml

helm-template:
	helm template orbit deploy/helm/orbit -n $(NAMESPACE) -f $(VALUES)

images:
	REGISTRY=$(REGISTRY) TAG=$(TAG) ./scripts/build-images.sh

push:
	REGISTRY=$(REGISTRY) TAG=$(TAG) ./scripts/push-images.sh

deploy:
	REGISTRY=$(REGISTRY) TAG=$(TAG) NAMESPACE=$(NAMESPACE) KUBE_CONTEXT=$(KUBE_CONTEXT) VALUES=$(VALUES) ./scripts/deploy.sh

smoke-k8s:
	API_URL=$(API_URL) ./scripts/smoke-k8s.sh
