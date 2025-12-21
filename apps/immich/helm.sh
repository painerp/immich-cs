#!/usr/bin/env sh

helm repo add cnpg https://cloudnative-pg.github.io/charts
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm repo update

helm upgrade --install cnpg --namespace cnpg-system --create-namespace cnpg/cloudnative-pg
helm upgrade --install prometheus  --namespace prometheus-system --create-namespace prometheus-community/kube-prometheus-stack
