# ID: lib/utils.js:194
const resolveTrustFn = (val) => {
  if (typeof val === 'function') return val;

  // trust everything
  if (val === true) return () => true;

  // trust by hop count
  if (typeof val === 'number') return (addr, hop) => hop < val;

  // comma-separated list of trusted addresses
  const subnets = typeof val === 'string'
    ? val.split(',').map((entry) => entry.trim())
    : val;

  return proxyaddr.compile(subnets || []);
};
