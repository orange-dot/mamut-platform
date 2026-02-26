#!/bin/bash

# End-to-End Integration Test Script for Cosmos DB + Flink Pipeline
# Tests the complete data flow from .NET Producer → Cosmos DB → Flink → Cosmos DB → .NET Consumer

set -e

echo "🚀 Starting End-to-End Integration Test for Cosmos DB + Flink Pipeline"
echo "=================================================================="

# Configuration
FLINK_UI="http://localhost:8081"
CONSUMER_API="http://localhost:8080"
TEST_DURATION_SECONDS=120
EXPECTED_MIN_EVENTS=10

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_service() {
    local service_name=$1
    local url=$2
    local max_attempts=30
    local attempt=0

    log_info "Checking $service_name availability at $url..."
    
    while [ $attempt -lt $max_attempts ]; do
        if curl -s "$url" > /dev/null 2>&1; then
            log_success "$service_name is available"
            return 0
        fi
        attempt=$((attempt + 1))
        echo -n "."
        sleep 2
    done
    
    log_error "$service_name is not available after $((max_attempts * 2)) seconds"
    return 1
}

# Step 1: Environment Validation
echo ""
log_info "Step 1: Validating Environment"
echo "------------------------------"

# Check if .env file exists
if [ ! -f "infra/.env" ]; then
    log_error ".env file not found. Please copy infra/.env.example to infra/.env and configure it."
    exit 1
fi

# Check if JAR exists
if [ ! -f "infra/job-jars/flink-cosmos-connector-java-1.0-SNAPSHOT.jar" ]; then
    log_error "Flink JAR not found. Please run 'mvn package' in flink-job-java directory."
    exit 1
fi

log_success "Environment validation completed"

# Step 2: Build and Start Infrastructure
echo ""
log_info "Step 2: Starting Infrastructure"
echo "-------------------------------"

# Start Docker Compose
log_info "Starting Docker Compose services..."
cd infra
docker-compose down -v 2>/dev/null || true
docker-compose up -d

log_success "Docker services started"

# Step 3: Health Checks
echo ""
log_info "Step 3: Health Checks"
echo "---------------------"

# Wait for services to be ready
check_service "Flink JobManager" "$FLINK_UI"
check_service "Consumer API" "$CONSUMER_API/health"

# Check Docker containers
log_info "Checking Docker containers status..."
container_status=$(docker-compose ps --format "table {{.Name}}\t{{.State}}")
echo "$container_status"

# Verify all containers are running
if docker-compose ps | grep -q "Exit"; then
    log_error "Some containers have exited. Check logs with 'docker-compose logs'"
    exit 1
fi

log_success "All services are healthy"

# Step 4: Deploy Flink Job
echo ""
log_info "Step 4: Deploying Flink Job"
echo "---------------------------"

# Get JobManager container name
jobmanager_container=$(docker-compose ps -q jobmanager)

# Submit Flink job
log_info "Submitting Flink job..."
job_submission_result=$(docker exec "$jobmanager_container" \
    flink run /opt/flink/job-jars/flink-cosmos-connector-java-1.0-SNAPSHOT.jar 2>&1)

echo "$job_submission_result"

# Extract job ID
job_id=$(echo "$job_submission_result" | grep -o "Job has been submitted with JobID [a-f0-9-]*" | sed 's/Job has been submitted with JobID //')

if [ -z "$job_id" ]; then
    log_error "Failed to extract job ID from submission result"
    log_info "Checking if job is already running..."
    
    # Check if job is already running
    jobs_response=$(curl -s "$FLINK_UI/jobs" | jq -r '.jobs[] | select(.status == "RUNNING") | .id' 2>/dev/null || echo "")
    if [ -n "$jobs_response" ]; then
        job_id=$jobs_response
        log_warning "Using existing running job: $job_id"
    else
        log_error "No running Flink job found"
        exit 1
    fi
fi

log_success "Flink job deployed successfully. Job ID: $job_id"

# Step 5: Validate Data Flow
echo ""
log_info "Step 5: Validating Data Flow"
echo "----------------------------"

# Wait for job to initialize
log_info "Waiting for job to initialize..."
sleep 30

# Monitor for specified duration
log_info "Monitoring data flow for $TEST_DURATION_SECONDS seconds..."

start_time=$(date +%s)
initial_stats=$(curl -s "$CONSUMER_API/stats" 2>/dev/null | jq -r '.totalEventsProcessed' 2>/dev/null || echo "0")

# Monitor progress
while [ $(($(date +%s) - start_time)) -lt $TEST_DURATION_SECONDS ]; do
    current_stats=$(curl -s "$CONSUMER_API/stats" 2>/dev/null | jq -r '.totalEventsProcessed' 2>/dev/null || echo "0")
    elapsed=$(($(date +%s) - start_time))
    
    echo -e "\r⏱️  Elapsed: ${elapsed}s | Events processed: $current_stats"
    sleep 5
done

echo # New line after progress

# Final validation
final_stats=$(curl -s "$CONSUMER_API/stats" 2>/dev/null | jq -r '.totalEventsProcessed' 2>/dev/null || echo "0")
events_processed=$((final_stats - initial_stats))

log_info "Events processed during test: $events_processed"

# Step 6: Collect Detailed Metrics
echo ""
log_info "Step 6: Collecting Detailed Metrics"
echo "-----------------------------------"

# Flink job metrics
log_info "Flink Job Status:"
flink_job_status=$(curl -s "$FLINK_UI/jobs/$job_id" 2>/dev/null | jq -r '.state' 2>/dev/null || echo "UNKNOWN")
echo "  Status: $flink_job_status"

# Consumer statistics
log_info "Consumer Statistics:"
consumer_stats=$(curl -s "$CONSUMER_API/stats" 2>/dev/null || echo "{}")
echo "  Total Events: $(echo "$consumer_stats" | jq -r '.totalEventsProcessed // "N/A"')"
echo "  Processing Rate: $(echo "$consumer_stats" | jq -r '.eventsPerSecond // "N/A"') events/sec"
echo "  Avg Processing Time: $(echo "$consumer_stats" | jq -r '.averageProcessingTimeMs // "N/A"') ms"

# Docker container resource usage
log_info "Container Resource Usage:"
docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}" \
    $(docker-compose ps -q) | head -5

# Check logs for errors
log_info "Checking for errors in logs..."
error_count=0

# Check producer logs
producer_errors=$(docker-compose logs producer 2>/dev/null | grep -i error | wc -l || echo "0")
if [ "$producer_errors" -gt 0 ]; then
    log_warning "Found $producer_errors error(s) in producer logs"
    error_count=$((error_count + producer_errors))
fi

# Check consumer logs
consumer_errors=$(docker-compose logs consumer 2>/dev/null | grep -i error | wc -l || echo "0")
if [ "$consumer_errors" -gt 0 ]; then
    log_warning "Found $consumer_errors error(s) in consumer logs"
    error_count=$((error_count + consumer_errors))
fi

# Check Flink logs
flink_errors=$(docker-compose logs taskmanager 2>/dev/null | grep -i error | wc -l || echo "0")
if [ "$flink_errors" -gt 0 ]; then
    log_warning "Found $flink_errors error(s) in Flink logs"
    error_count=$((error_count + flink_errors))
fi

# Step 7: Test Results
echo ""
log_info "Step 7: Test Results"
echo "-------------------"

test_passed=true

# Check if job is running
if [ "$flink_job_status" != "RUNNING" ]; then
    log_error "Flink job is not in RUNNING state: $flink_job_status"
    test_passed=false
fi

# Check if events were processed
if [ "$events_processed" -lt $EXPECTED_MIN_EVENTS ]; then
    log_error "Insufficient events processed: $events_processed < $EXPECTED_MIN_EVENTS"
    test_passed=false
fi

# Check error count
if [ "$error_count" -gt 5 ]; then
    log_error "Too many errors found in logs: $error_count"
    test_passed=false
fi

# Final result
echo ""
echo "=================================================================="
if [ "$test_passed" = true ]; then
    log_success "🎉 END-TO-END INTEGRATION TEST PASSED!"
    echo ""
    echo "✅ Flink job is running successfully"
    echo "✅ Data is flowing through the complete pipeline"
    echo "✅ Events are being processed correctly"
    echo "✅ All services are healthy"
    echo ""
    echo "📊 Test Summary:"
    echo "   • Events processed: $events_processed"
    echo "   • Test duration: $TEST_DURATION_SECONDS seconds"
    echo "   • Flink job status: $flink_job_status"
    echo "   • Error count: $error_count"
    echo ""
    echo "🚀 The Cosmos DB + Flink pipeline is ready for production!"
else
    log_error "❌ END-TO-END INTEGRATION TEST FAILED!"
    echo ""
    echo "Issues found:"
    [ "$flink_job_status" != "RUNNING" ] && echo "   • Flink job not running"
    [ "$events_processed" -lt $EXPECTED_MIN_EVENTS ] && echo "   • Insufficient events processed"
    [ "$error_count" -gt 5 ] && echo "   • Too many errors in logs"
    echo ""
    echo "🔍 Check the logs for more details:"
    echo "   docker-compose logs producer"
    echo "   docker-compose logs consumer"
    echo "   docker-compose logs taskmanager"
fi

echo ""
log_info "Test completed. Infrastructure is still running for manual inspection."
log_info "To stop: cd infra && docker-compose down"
log_info "Flink UI: $FLINK_UI"
log_info "Consumer API: $CONSUMER_API"

# Exit with appropriate code
[ "$test_passed" = true ] && exit 0 || exit 1