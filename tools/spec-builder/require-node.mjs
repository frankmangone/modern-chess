// Run first by the Makefile, in plain JavaScript so that an old Node can say what is wrong: the
// builder is TypeScript that Node runs directly, which only Node 24 and newer does without a flag.
const major = Number(process.versions.node.split(".")[0]);
if (major < 24) {
  console.error(`the spec builder needs Node 24 or newer to run TypeScript directly (this is ${process.version})`);
  process.exit(1);
}
