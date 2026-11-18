// An addon whose view functions all fail with the panic status.
const panic = () => ({ status: 2 });
module.exports = {
  version: () => ({ status: 0, value: '0.0.0' }),
  treePath: panic,
  treeBytes: panic,
  describeSheetPath: panic,
  describeSheetBytes: panic,
};
