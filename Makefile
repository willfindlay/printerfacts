# SPDX-License-Identifier: MIT
#
# Distributed printer facts in Rust, inspired by Christine Dodrill.
# Copyright (c) 2021  William Findlay
#
# September 25, 2021  William Findlay  Created this.

DOCKER_FLAGS =

IMAGE_NAME = wpfindlay/printerfacts:latest

MANIFEST_TEMPLATE = templates/deploy.yml
MANIFEST = manifest/deploy.yml

.PHONY: manifest
manifest: $(MANIFEST)

.PHONY: build
build: $(MANIFEST)
	@docker build $(DOCKER_FLAGS) -t "$(IMAGE_NAME)" .

.PHONY: run-local
run-local: build
	@docker run -it -p 4000:4000 --rm "$(IMAGE_NAME)"

.PHONY: push
push:
	@docker push "$(IMAGE_NAME)"

$(MANIFEST): $(MANIFEST_TEMPLATE)
	@sed -e '' "$(MANIFEST_TEMPLATE)" > "$(MANIFEST)"
	@echo "Production manifest is located at $(MANIFEST)"

# vi:ft=make
