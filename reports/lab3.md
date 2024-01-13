## 实现功能
1. 实现spawn系统调用，和fork类似，会创建一个新的子进程，但是该子进程的内存空间为从ELF文件加载的全新的内存空间，和调用进程的关系只有父子进程关系。
2. 实现stride调度算法，在TaskControlBlock中加入priority存放优先级信息，以及stride存放当前进程的stride。TASK_MANAGER中寻找下一个调度的进程时，选择stride最小的，并将该进程的stride加上BIG_STRIDE/priority。BIG_STRIDE设置为214748，足够大又不至于溢出导致优先级反转。

## 问答题

1. 不是。由于p2.stride为u8类型，p2.stride + pass = 260 > 255，产生溢出，p2.stride变为4，导致下一次调度时stride较小的进程为p2。
2. 初始时stride为0，显然STRIDE_MAX – STRIDE_MIN <= BigStride / 2。假设某个时刻满足STRIDE_MAX – STRIDE_MIN <= BigStride / 2，进行调度，由于优先级全部>=2，因此每次调度的pass最多为BigStride / 2。每次调度时选取的是stride最小的进程，因此增加pass后也满足STRIDE_MAX – STRIDE_MIN <= BigStride / 2。

```rust
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let mut diff: u64 = 0;
        if(self.0 < other.0){
            diff = other.0 - self.0;
        }
        else{
            diff = self.0 - other.0;
        }
        if diff <= BigStride / 2{
            self.0.partial_cmp(&other.0)
        }
        else{
            other.0.partial_cmp(&self.0)
        }
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}
```

## 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

>无

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

>无

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。