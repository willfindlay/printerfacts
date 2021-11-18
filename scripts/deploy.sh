#! /bin/sh

set -xeu

MANIFEST="$1"

kubectl apply -f "$MANIFEST"
kubectl rollout restart deployment comp4000server
