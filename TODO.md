
1. test IntervalTree single delete
1. rename delete_range to drain_range and add remove_range that throws the data away
1. find, query_range, query_overlap
1. make conv_to_tuple_vec() less unsafe, check all transmutes
1. benchmark bulk delete with 10 items at a time
1. IntervalTree benchmarks
1. reference implementation + benchmarks
1. comparative performance graphs of variations (with/without filtering, intervals, etc)
1. benchmarks in comparison to typical explicit-array-based implementation
1. full usage example in Readme
1. implement insertion with rebuilding/reallocing when the target index is out of bounds