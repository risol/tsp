#!/bin/bash
# Samba AD test user initialization script
# This script runs automatically after container starts

# Wait for Samba to fully start
echo "Waiting for Samba AD service to start..."
sleep 30

# Check if service is ready
until samba-tool domain level show 2>/dev/null; do
    echo "Waiting for Samba AD to be ready..."
    sleep 5
done

echo "Samba AD is ready, starting to create test users..."

# Create test users (not specifying givenName/surname to ensure cn matches username)
echo "Creating test user testuser1..."
samba-tool user create testuser1 Test@123 --server=localhost 2>/dev/null || echo "User testuser1 may already exist"

echo "Creating test user testuser2..."
samba-tool user create testuser2 Test@123 --server=localhost 2>/dev/null || echo "User testuser2 may already exist"

echo "Creating test user testuser3..."
samba-tool user create testuser3 Test@123 --server=localhost 2>/dev/null || echo "User testuser3 may already exist"

# Create test group
echo "Creating test group TestGroup..."
samba-tool group add TestGroup --server=localhost 2>/dev/null || echo "Group TestGroup may already exist"

# Add users to group
echo "Adding users to group..."
samba-tool group addmembers TestGroup testuser1,testuser2 --server=localhost 2>/dev/null || echo "Users may already be in group"

echo "Test user creation complete!"
echo ""
echo "Test user information:"
echo "  Username: testuser1, testuser2, testuser3"
echo "  Password: Test@123"
echo "  DN: CN=testuser1,CN=Users,DC=example,DC=com"
echo "      CN=testuser2,CN=Users,DC=example,DC=com"
echo "      CN=testuser3,CN=Users,DC=example,DC=com"
