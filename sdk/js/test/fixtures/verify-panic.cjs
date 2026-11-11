// An addon whose verify functions all fail with the panic status.
const panic = () => ({ status: 2 });
module.exports = {
  version: () => ({ status: 0, value: '0.0.0' }),
  verifyPath: panic,
  verifyBytes: panic,
  checkPolicy: panic,
};
