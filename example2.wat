(module
  (func $add (param $lhs i32) (param $rhs i32) (param $th i32) (result i32)
    local.get $lhs
    local.get $rhs
    i32.add
    local.get $th
    i32.mul)
  (export "add" (func $add))
)
