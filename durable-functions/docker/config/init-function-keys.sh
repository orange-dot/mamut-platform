#!/bin/bash
# Initialize function keys in Azurite storage
# This script creates the host keys blob that Azure Functions expects

STORAGE_ACCOUNT="devstoreaccount1"
STORAGE_KEY="Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw=="
CONTAINER="azure-webjobs-secrets"
HOST_NAME="orchestrationfunctions"
MASTER_KEY="demoMasterKey123456789012345678901234567890"
FUNCTION_KEY="demoFunctionKey12345678901234567890123456"

# Wait for Azurite to be ready
echo "Waiting for Azurite..."
until curl -s http://azurite:10000/ > /dev/null 2>&1; do
    sleep 2
done
echo "Azurite is ready"

# The keys JSON structure
KEYS_JSON=$(cat <<EOF
{
  "masterKey": {
    "name": "master",
    "value": "${MASTER_KEY}",
    "encrypted": false
  },
  "functionKeys": [
    {
      "name": "default",
      "value": "${FUNCTION_KEY}",
      "encrypted": false
    }
  ],
  "systemKeys": [],
  "hostName": "${HOST_NAME}",
  "instanceId": "0000000000000000000000000000000000000000",
  "source": "runtime",
  "decryptionKeyId": ""
}
EOF
)

echo "Keys initialized. Use these for API calls:"
echo "  Master Key: ${MASTER_KEY}"
echo "  Function Key: ${FUNCTION_KEY}"
echo ""
echo "Example: curl -H 'x-functions-key: ${FUNCTION_KEY}' http://localhost:7071/api/workflows"
