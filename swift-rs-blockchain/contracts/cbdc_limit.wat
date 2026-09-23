(module
  (func (export "within_limit") (param $amount i64) (param $limit i64) (result i32)
    local.get $amount
    local.get $limit
    i64.le_u))
