#!/bin/bash

# Colors for pretty output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
DATABASE_URL="postgresql://dev:dev@localhost/bigtwo"
JWT_SECRET="dev-secret-change-in-production"

# Helper functions
log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Check if Docker is running
check_docker() {
    if ! docker info >/dev/null 2>&1; then
        log_error "Docker is not running. Please start Docker Desktop."
        exit 1
    fi
    log_success "Docker is running"
}

# Check if PostgreSQL container is running
check_postgres() {
    if docker compose -f docker-compose.dev.yml ps postgres | grep -q "Up"; then
        log_success "PostgreSQL container is already running"
        return 0
    else
        log_info "PostgreSQL container is not running"
        return 1
    fi
}

# Start PostgreSQL if not running
start_postgres() {
    log_info "Starting PostgreSQL container..."
    docker compose -f docker-compose.dev.yml up -d postgres
    
    # Wait for PostgreSQL to be ready
    log_info "Waiting for PostgreSQL to be ready..."
    local max_attempts=30
    local attempt=1
    
    while [ $attempt -le $max_attempts ]; do
        if docker compose -f docker-compose.dev.yml exec postgres pg_isready -U dev -d bigtwo >/dev/null 2>&1; then
            log_success "PostgreSQL is ready!"
            return 0
        fi
        echo -n "."
        sleep 1
        attempt=$((attempt + 1))
    done
    
    log_error "PostgreSQL failed to start after ${max_attempts} seconds"
    exit 1
}

# Run database migrations
run_migrations() {
    log_info "Running database migrations..."
    export DATABASE_URL="$DATABASE_URL"
    
    if sqlx migrate run; then
        log_success "Migrations completed successfully"
    else
        log_error "Migration failed"
        exit 1
    fi
}

# Check if sqlx-cli is installed
check_sqlx() {
    if ! command -v sqlx >/dev/null 2>&1; then
        log_warning "sqlx-cli not found. Installing..."
        cargo install sqlx-cli --no-default-features --features native-tls,postgres
    fi
}

# Kill any existing server process
cleanup_server() {
    local pids=$(lsof -ti:3000 2>/dev/null)
    if [ ! -z "$pids" ]; then
        log_info "Killing existing server process on port 3000..."
        kill $pids 2>/dev/null || true
        sleep 2
    fi
}

# Main execution
main() {
    echo -e "${BLUE}🚀 Big Two Development Server${NC}"
    echo "=================================="
    
    # Parse command line arguments
    USE_POSTGRES=false
    SKIP_BUILD=false
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --postgres|-p)
                USE_POSTGRES=true
                shift
                ;;
            --memory|-m)
                USE_POSTGRES=false
                shift
                ;;
            --skip-build|-s)
                SKIP_BUILD=true
                shift
                ;;
            --help|-h)
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  -p, --postgres     Use PostgreSQL (persistent sessions)"
                echo "  -m, --memory       Use in-memory storage (default)"
                echo "  -s, --skip-build   Skip cargo build check"
                echo "  -h, --help         Show this help message"
                echo ""
                echo "Examples:"
                echo "  $0                 # Run with in-memory storage"
                echo "  $0 --postgres      # Run with PostgreSQL"
                echo "  $0 -p -s           # PostgreSQL + skip build"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                echo "Use --help for usage information"
                exit 1
                ;;
        esac
    done
    
    # Clean up any existing server
    cleanup_server
    
    if [ "$USE_POSTGRES" = true ]; then
        log_info "Setting up PostgreSQL development environment..."
        
        # Check dependencies
        check_docker
        check_sqlx
        
        # Setup PostgreSQL
        if ! check_postgres; then
            start_postgres
        fi
        
        # Run migrations
        run_migrations
        
        # Set environment variables
        export DATABASE_URL="$DATABASE_URL"
        export JWT_SECRET="$JWT_SECRET"
        
        log_success "PostgreSQL setup complete"
        log_info "Using persistent session storage"
    else
        log_info "Using in-memory session storage (fast development mode)"
        # Unset DATABASE_URL to ensure in-memory mode
        unset DATABASE_URL
        export JWT_SECRET="$JWT_SECRET"
    fi
    
    # Build check (unless skipped)
    if [ "$SKIP_BUILD" = false ]; then
        log_info "Checking build..."
        if ! cargo check --quiet; then
            log_error "Build check failed. Fix compilation errors first."
            exit 1
        fi
        log_success "Build check passed"
    fi
    
    # Start the server
    echo ""
    log_success "🔥 Starting Big Two server..."
    echo -e "${YELLOW}Press Ctrl+C to stop${NC}"
    echo "=================================="
    
    # Run the server
    exec cargo run
}

# Run main function with all arguments
main "$@" 