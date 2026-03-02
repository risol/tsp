#!/bin/bash
# Samba AD user fields demo (using actually supported fields)
# Based on available options in samba-tool user create --help

set -e

LDAP_CONTAINER="tsp-samba-ad"
ADMIN_DN="CN=Administrator,CN=Users,DC=example,DC=com"
ADMIN_PASS="P@ssw0rd123"

echo "=========================================="
echo "  Samba AD User Fields Demo"
echo "=========================================="
echo ""

# First delete any existing test users
echo "Cleaning old users..."
docker exec $LDAP_CONTAINER samba-tool user delete zhangsan -H ldap://localhost -U "Administrator%${ADMIN_PASS}" 2>/dev/null || echo "zhangsan does not exist"
docker exec $LDAP_CONTAINER samba-tool user delete lisi -H ldap://localhost -U "Administrator%${ADMIN_PASS}" 2>/dev/null || echo "lisi does not exist"
docker exec $LDAP_CONTAINER samba-tool user delete wangwu -H ldap://localhost -U "Administrator%${ADMIN_PASS}" 2>/dev/null || echo "wangwu does not exist"

echo ""
echo "Creating users with samba-tool supported fields..."
echo ""

# Create zhangsan - Software Engineer
echo "Creating zhangsan (Software Engineer)..."
docker exec $LDAP_CONTAINER samba-tool user create zhangsan Test@123 \
    --use-username-as-cn \
    --given-name=Zhang \
    --surname=San \
    --initials=ZS \
    --mail-address=zhangsan@example.com \
    --job-title="Senior Software Engineer" \
    --department="R&D" \
    --company="Example Tech Co" \
    --description="Responsible for core backend system architecture design" \
    --telephone-number="010-12345678" \
    --internet-address="http://blog.example.com" \
    --physical-delivery-office="Beijing HQ" \
    --home-directory=/home/zhangsan \
    --login-shell=/bin/bash \
    -H ldap://localhost \
    -U "Administrator%${ADMIN_PASS}" 2>&1 | grep -E "User|Added" | head -1

# Create lisi - DevOps Engineer
echo "Creating lisi (DevOps Engineer)..."
docker exec $LDAP_CONTAINER samba-tool user create lisi Test@123 \
    --use-username-as-cn \
    --given-name=Li \
    --surname=Si \
    --initials=LS \
    --mail-address=lisi@example.com \
    --job-title="DevOps Engineer" \
    --department="Operations" \
    --company="Example Tech Co" \
    --description="Responsible for CI/CD and container orchestration" \
    --telephone-number="021-87654321" \
    --internet-address="https://github.com/lisi" \
    --physical-delivery-office="Shanghai Branch" \
    --home-directory=/home/lisi \
    --login-shell=/bin/bash \
    -H ldap://localhost \
    -U "Administrator%${ADMIN_PASS}" 2>&1 | grep -E "User|Added" | head -1

# Create wangwu - Product Manager
echo "Creating wangwu (Product Manager)..."
docker exec $LDAP_CONTAINER samba-tool user create wangwu Test@123 \
    --use-username-as-cn \
    --given-name=Wang \
    --surname=Wu \
    --initials=WW \
    --mail-address=wangwu@example.com \
    --job-title="Product Manager" \
    --department="Product" \
    --company="Example Tech Co" \
    --description="Responsible for product planning and user research" \
    --telephone-number="0755-11112222" \
    --internet-address="http://www.example.com/~wangwu" \
    --physical-delivery-office="Guangzhou Branch" \
    --home-directory=/home/wangwu \
    --login-shell=/bin/bash \
    -H ldap://localhost \
    -U "Administrator%${ADMIN_PASS}" 2>&1 | grep -E "User|Added" | head -1

echo ""
echo "=========================================="
echo "  ✅ User creation complete"
echo "=========================================="
echo ""

# View created user details
echo "User details:"
echo ""
echo "1. zhangsan:"
docker exec $LDAP_CONTAINER samba-tool user show zhangsan -H ldap://localhost -U "Administrator%${ADMIN_PASS}" 2>&1 | grep -E "^dn:|^cn:|^givenName:|^sn:|^initials:|^mail:|^telephoneNumber:|^department:|^company:|^title:|^description:" | head -12

echo ""
echo "2. lisi:"
docker exec $LDAP_CONTAINER samba-tool user show lisi -H ldap://localhost -U "Administrator%${ADMIN_PASS}" 2>&1 | grep -E "^dn:|^cn:|^givenName:|^sn:|^initials:|^mail:|^telephoneNumber:|^department:|^company:|^title:|^description:" | head -12

echo ""
echo "=========================================="
echo "  samba-tool supported fields"
echo "=========================================="
echo ""
echo "✅ Basic Information:"
echo "    --use-username-as-cn (force use username as CN)"
echo "    --given-name (given name)"
echo "    --surname (surname)"
echo "    --initials (initials)"
echo ""
echo "✅ Contact Information:"
echo "    --mail-address (email)"
echo "    --telephone-number (office phone)"
echo "    --internet-address (personal homepage)"
echo ""
echo "✅ Work Information:"
echo "    --company (company)"
echo "    --department (department)"
echo "    --job-title (job title)"
echo "    --description (description)"
echo "    --physical-delivery-office (office location)"
echo ""
echo "✅ Unix Attributes:"
echo "    --home-directory (home directory)"
echo "    --login-shell (login shell)"
echo ""
echo "❌ Not supported but can be added via ldapmodify:"
echo "    - mobile (mobile number)"
echo "    - displayName (display name, auto-composed)"
echo "    - streetAddress (street address)"
echo "    - l (city)"
echo "    - st (state/province)"
echo "    - postalCode (postal code)"
echo "    - co (country code)"
echo ""
echo "=========================================="
echo "  View LDAP attributes"
echo "=========================================="
echo ""
echo "View all attributes:"
echo "  docker exec tsp-samba-ad ldapsearch -x \\"
echo "    -H ldap://localhost:389 \\"
echo "    -b CN=Users,DC=example,DC=com \\"
echo "    -D 'CN=Administrator,CN=Users,DC=example,DC=com' \\"
echo "    -w P@ssw0rd123 \\"
echo "    '(cn=zhangsan)'"
echo ""
