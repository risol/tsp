#!/bin/bash
deno compile \
  --allow-net \
  --allow-read \
  --allow-write \
  --allow-env \
  --output tsp-fpm-test \
  src/main.ts

echo "Starting server..."
./tsp-fpm-test --root ./tests/test_www --port 9100 --dev &
SERVER_PID=$!

sleep 3

echo "Testing injection.tsx..."
curl -v http://localhost:9100/injection.tsx

echo ""
echo "Server logs:"
sleep 1
kill $SERVER_PID 2>/dev/null
