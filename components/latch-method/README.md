# `latch-method`

HTTP latch that makes decisions based on the request's method.

Authorization is granted or denied based on the wasi:config with the lowercase method value as the config key, and `granted`, `denied`, or `abstained` as the value. A default decision may specified under the `*` key.

## The `latch-method` World

- imports `wasi:config/store@0.2`
- exports `componentized:http/latch`
