---
name: tsp-ldap
description: LDAP directory operations for TSP. Use when connecting to LDAP/Active Directory for authentication or user lookup.
---

# TSP LDAP

Use this skill for LDAP directory operations in TSP.

## LDAP Usage

```typescript
export default Page(async function(ctx, { createLdap, response }) {
  const ldap = await createLdap({
    url: 'ldap://127.0.0.1:389',
    bindDN: 'cn=admin,dc=example,dc=org',
    bindCredentials: 'admin_password',
    baseDN: 'dc=example,dc=org'
  });

  // Search users
  const users = await ldap.search('dc=example,dc=org', {
    filter: '(objectClass=person)',
    scope: 'sub',
    attributes: ['cn', 'mail', 'uid']
  });
  // Returns: LdapEntry[]

  // Search with custom filter
  const admins = await ldap.search('dc=example,dc=org', {
    filter: '(&(objectClass=person)(memberOf=cn=admins,dc=example,dc=org))',
    scope: 'sub',
    attributes: ['cn', 'mail']
  });

  // Get single entry
  const user = await ldap.searchOne('uid=john,ou=users,dc=example,dc=org', {
    attributes: ['cn', 'mail', 'uid']
  });

  // Add entry
  await ldap.add('uid=newuser,ou=users,dc=example,dc=org', {
    objectClass: ['person', 'inetOrgPerson'],
    cn: 'New User',
    sn: 'User',
    uid: 'newuser',
    mail: 'newuser@example.com'
  });

  // Modify entry
  await ldap.modify('uid=john,ou=users,dc=example,dc=org', [
    { operation: 'add', modification: { mail: 'newemail@example.com' } },
    { operation: 'replace', modification: { cn: 'John Doe' } },
    { operation: 'delete', modification: { telephoneNumber: [] } }
  ]);

  // Delete entry
  await ldap.del('uid=olduser,ou=users,dc=example,dc=org');

  // Bind (authenticate)
  const isAuthenticated = await ldap.bind('uid=john,ou=users,dc=example,dc=org', 'password123');

  return response.json({
    userCount: users.length,
    adminsCount: admins.length,
    authenticated: isAuthenticated
  });
});
```

## Key Methods

| Method | Description |
|--------|-------------|
| `search()` | Search directory |
| `searchOne()` | Get single entry |
| `add()` | Add new entry |
| `modify()` | Modify entry |
| `del()` | Delete entry |
| `bind()` | Authenticate |

## Search Options

```typescript
const results = await ldap.search('dc=example,dc=org', {
  // Filter (LDAP filter syntax)
  filter: '(&(objectClass=person)(mail=*))',

  // Scope: 'base', 'one', 'sub'
  scope: 'sub',

  // Attributes to return
  attributes: ['cn', 'mail', 'uid', 'memberOf'],

  // Limit results
  sizeLimit: 100,

  // Return attribute types only
  typesOnly: false
});
```

## Common LDAP Filters

| Filter | Meaning |
|--------|---------|
| `(objectClass=*)` | All entries |
| `(cn=John*)` | Name starts with John |
| `(mail=*@example.com)` | Email at example.com |
| `(&(department=IT)(memberOf=cn=admins))` | IT dept AND admin group |
| `(|(cn=John)(cn=Jane))` | John OR Jane |

## Best Practices

- Always specify attributes to reduce bandwidth
- Use baseDN to limit search scope
- Close LDAP connection after use (TSP handles this automatically)
- Use connection pooling for high traffic
- Handle LDAP errors gracefully (connection failures, timeout)
