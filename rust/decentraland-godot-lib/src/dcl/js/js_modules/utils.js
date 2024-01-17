module.exports.run_async = async function () {
    await Deno.core.ops.op_run_async();
}