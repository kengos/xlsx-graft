// An addon whose view functions all fail with the invalid-argument status.
const refuse = () => ({ status: 4 });
module.exports = {
  version: () => ({ status: 0, value: '0.0.0' }),
  treePath: refuse,
  treeBytes: refuse,
  describeSheetPath: refuse,
  describeSheetBytes: refuse,
};
