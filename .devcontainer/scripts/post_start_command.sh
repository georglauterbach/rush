#! /usr/bin/env bash

set -eE -u -o pipefail
shopt -s inherit_errexit

echo 'Managing VS Code workspace configuration'
mkdir -p "${WORKSPACE_DIR_CONTAINER}/.vscode"
cp --update=none "${WORKSPACE_DIR_CONTAINER}/.devcontainer/vscode/"* "${WORKSPACE_DIR_CONTAINER}/.vscode/"

echo 'Creating host path in container for Cargo to display proper links'
sudo mkdir -p "$(dirname "${WORKSPACE_DIR_HOST}")"
sudo ln -sf "${WORKSPACE_DIR_CONTAINER}" "${WORKSPACE_DIR_HOST}"
