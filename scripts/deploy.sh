#! /bin/sh

set -xeu

MANIFEST="$1"
MIGRATIONS="$2"

kubectl apply -f "$MIGRATIONS"
kubectl apply -f "$MANIFEST"
kubectl rollout restart deployment server
