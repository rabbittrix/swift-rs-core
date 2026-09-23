(module
  (func (export "quorum_met") (param $votes i64) (param $total i64) (result i32)
    (i64.ge_u
      (i64.mul (local.get $votes) (i64.const 100))
      (i64.mul (local.get $total) (i64.const 67)))))
