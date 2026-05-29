// Allocation-heavy program to trigger frequent GCs, so javagc has pauses to
// time. Run with a small heap to force collection, e.g.:
//   java -Xmx64m -XX:+UseG1GC Alloc
public class Alloc {
    public static void main(String[] args) throws InterruptedException {
        System.out.println("Alloc pid " + ProcessHandle.current().pid() + " — allocating to trigger GC");
        java.util.ArrayList<byte[]> live = new java.util.ArrayList<>();
        long i = 0;
        while (true) {
            // churn: allocate, keep some, drop most
            byte[] b = new byte[64 * 1024];
            if ((i & 7) == 0) live.add(b);
            if (live.size() > 256) live.clear();   // let a batch become garbage
            if ((i % 1000) == 0) Thread.sleep(1);
            i++;
        }
    }
}
