#!/bin/bash

set -e

echo "🚀 Starting test database..."
docker-compose up -d db

echo "⏳ Waiting for database to be ready..."
until docker-compose exec -T db pg_isready -U postgres > /dev/null 2>&1; do
  sleep 0.5
done

echo "✅ Database is ready."

echo "🛠️ Running migrations..."
sqlx migrate run --ignore-missing

echo "📦 Running tests..."
DATABASE_URL=postgres://postgres:password@localhost:5432/activity_db \
  cargo test

docker-compose down