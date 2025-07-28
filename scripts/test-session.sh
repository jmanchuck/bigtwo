#!/bin/bash

echo "🔍 Testing Big Two API endpoints..."

# Test session creation
echo "1. Creating session..."
SESSION_RESPONSE=$(curl -s -X POST http://localhost:3000/session)
echo "Session response: $SESSION_RESPONSE"

# Extract session_id from JSON response
SESSION_ID=$(echo $SESSION_RESPONSE | python3 -c "import sys, json; print(json.load(sys.stdin)['session_id'])" 2>/dev/null)

if [ -z "$SESSION_ID" ]; then
    echo "❌ Failed to create session or extract session_id"
    exit 1
fi

echo "✅ Session ID: ${SESSION_ID:0:20}..."

# Test session validation
echo ""
echo "2. Validating session..."
curl -s -H "X-Session-ID: $SESSION_ID" http://localhost:3000/session/validate
echo ""

# Create a room
echo ""
echo "3. Creating room..."
ROOM_RESPONSE=$(curl -s -X POST http://localhost:3000/room -H "Content-Type: application/json" -d '{"host_name": "test-host"}')
echo "Room response: $ROOM_RESPONSE"

# Extract room_id
ROOM_ID=$(echo $ROOM_RESPONSE | python3 -c "import sys, json; print(json.load(sys.stdin)['id'])" 2>/dev/null)

if [ -z "$ROOM_ID" ]; then
    echo "❌ Failed to create room or extract room_id"
    exit 1
fi

echo "✅ Room ID: $ROOM_ID"

# Test joining room
echo ""
echo "4. Joining room..."
JOIN_RESPONSE=$(curl -s -w "\nHTTP_STATUS:%{http_code}\n" -X POST "http://localhost:3000/room/$ROOM_ID/join" \
    -H "Content-Type: application/json" \
    -H "X-Session-ID: $SESSION_ID" \
    -d '{}')

echo "Join response: $JOIN_RESPONSE"

echo ""
echo "🏁 Test complete!" 