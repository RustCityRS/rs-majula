package majula;

import java.awt.AWTEvent;
import java.awt.EventQueue;
import java.awt.Toolkit;
import java.awt.event.InputEvent;
import java.awt.event.MouseEvent;
import java.awt.event.MouseWheelEvent;
import java.io.File;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.net.URL;
import java.net.URLClassLoader;
import java.util.Arrays;
import java.util.HashSet;
import java.util.Hashtable;
import java.util.Locale;
import java.util.Set;

// Starts the 530 client beside this jar (client.jar, or the HD runescape_gl.jar) the way the 2009
// loader did: the gamepack gets its own class loader, with the signlink classes taken from the loader
// jar. Only the loader's signlink saves preferences and keeps a disk cache. Both go under a home
// folder of 530's own, so other revisions' clients can't overwrite them. Built for Java 5, so it
// also runs on the Java versions that old Macs top out at.
public class Launch530 {
    private static final String[] SD_SHARED = {"fb", "dj", "ne", "eh", "k"};
    private static final String[] HD_SHARED = {"ll", "im", "qm", "ad", "ai"};
    private static final int MAX_INDEXES = 64;

    public static void main(String[] args) throws Throwable {
        File dir = new File(Launch530.class.getProtectionDomain().getCodeSource().getLocation().toURI()).getParentFile();
        File glJar = new File(dir, "runescape_gl.jar");
        boolean hd = glJar.exists();

        File home = new File(System.getProperty("user.home"), ".rs-majula" + File.separator + "530");
        home.mkdirs();
        System.setProperty("user.home", home.getPath());

        rightClickAsMeta();
        boolean gl = hd && loadGl(new File(dir, "natives"));
        pinCacheFiles(hd ? "ll" : "fb", hd ? "w" : "a", new File(home, "cache"));
        Set<String> shared = new HashSet<String>(Arrays.asList(hd ? HD_SHARED : SD_SHARED));
        ClassLoader game = new GameLoader(hd ? glJar : new File(dir, "client.jar"), shared, Launch530.class.getClassLoader());
        if (gl) {
            // The client only enables GL (rc.M) once its signlink has loaded the JOGL natives, and the
            // loader's way of loading them no longer works on current Java, so they are loaded above.
            Field available = game.loadClass("rc").getDeclaredField("M");
            available.setAccessible(true);
            available.setBoolean(null, true);
        }
        game.loadClass("client").getMethod("main", String[].class).invoke(null, (Object) args);
    }

    // The folder under natives/ holding the JOGL build for this computer, picked as the 2009 loader
    // picked it, except that 64-bit SPARC gets its own build (the loader handed it the 32-bit one)
    // and processors JOGL had no build for, such as ARM, get none (the loader assumed x86).
    static String platform(String os, String arch) {
        boolean wide = arch.startsWith("amd64") || arch.startsWith("x86_64");
        boolean x86 = arch.equals("x86") || arch.matches("i[3-6]86");
        if (os.startsWith("win")) {
            return wide ? "windows-amd64" : x86 ? "windows-x86" : null;
        }
        if (os.startsWith("linux")) {
            return wide ? "linux-amd64" : x86 ? "linux-x86" : null;
        }
        if (os.startsWith("mac")) {
            if (arch.startsWith("ppc")) {
                return "macos-powerpc";
            }
            return wide || x86 ? "macos-universal" : null;
        }
        if (os.startsWith("sunos")) {
            if (wide) {
                return "solaris-amd64";
            }
            if (arch.startsWith("sparcv9")) {
                return "solaris-sparcv9";
            }
            if (arch.startsWith("sparc")) {
                return "solaris-sparc";
            }
            return x86 ? "solaris-x86" : null;
        }
        return null;
    }

    // Loads JOGL's natives in the 2009 loader's order: the JDK's jawt first (except on macOS, where
    // JOGL links Apple's own), and on Linux and Solaris gluegen-rt, then JOGL inside DRIHack, which
    // opens the system libGL globally so DRI drivers resolve. Without GL the client offers standard
    // detail only.
    static boolean loadGl(File natives) {
        String os = System.getProperty("os.name").toLowerCase(Locale.ENGLISH);
        String arch = System.getProperty("os.arch").toLowerCase(Locale.ENGLISH);
        String platform = platform(os, arch);
        if (platform == null) {
            System.err.println("High detail has no natives for " + os + " " + arch + "; standard detail only.");
            return false;
        }
        File libs = new File(natives, platform);
        try {
            if (!os.startsWith("mac")) {
                System.loadLibrary("jawt");
            }
            if (os.startsWith("win")) {
                load(libs, "jogl.dll");
                load(libs, "jogl_awt.dll");
            } else if (os.startsWith("mac")) {
                load(libs, "libjogl.jnilib");
                load(libs, "libjogl_awt.jnilib");
            } else {
                load(libs, "libgluegen-rt.so");
                Class<?> dri = Class.forName("com.sun.opengl.impl.x11.DRIHack");
                Method begin = dri.getMethod("begin");
                Method end = dri.getMethod("end");
                begin.invoke(null);
                load(libs, "libjogl.so");
                end.invoke(null);
                load(libs, "libjogl_awt.so");
            }
            return true;
        } catch (Throwable t) {
            System.err.println("High detail could not load " + platform + " natives (" + t + "); standard detail only.");
            return false;
        }
    }

    static void load(File libs, String name) {
        System.load(new File(libs, name).getAbsolutePath());
    }

    // The signlink finds each cache file in the first existing .jagex_cache_32 out of c:/rscache/, c:/windows/,
    // c:/ and then the home folder, which every revision's client shares, so they overwrite each other's
    // caches. It looks a file name up in its static name -> File table first, so fill that in beforehand.
    @SuppressWarnings("unchecked")
    static void pinCacheFiles(String signlink, String table, File cache) throws Exception {
        cache.mkdirs();
        Field field = Class.forName(signlink, true, Launch530.class.getClassLoader()).getDeclaredField(table);
        field.setAccessible(true);
        Hashtable<String, File> files = (Hashtable<String, File>) field.get(null);
        files.put("random.dat", new File(cache, "random.dat"));
        files.put("main_file_cache.dat2", new File(cache, "main_file_cache.dat2"));
        files.put("main_file_cache.idx255", new File(cache, "main_file_cache.idx255"));
        for (int i = 0; i < MAX_INDEXES; i++) {
            files.put("main_file_cache.idx" + i, new File(cache, "main_file_cache.idx" + i));
        }
    }

    // The client reads a right click as MouseEvent.isMetaDown(), which Java 9 and later no longer set
    // for the right button, so give right-button mouse events the meta flag before it sees them.
    static void rightClickAsMeta() {
        Toolkit.getDefaultToolkit().getSystemEventQueue().push(new EventQueue() {
            @Override
            protected void dispatchEvent(AWTEvent event) {
                if (event instanceof MouseEvent && !(event instanceof MouseWheelEvent)) {
                    MouseEvent mouse = (MouseEvent) event;
                    int ex = mouse.getModifiersEx();
                    if ((ex & InputEvent.BUTTON3_DOWN_MASK) != 0 && (ex & InputEvent.META_DOWN_MASK) == 0) {
                        event = new MouseEvent(mouse.getComponent(), mouse.getID(), mouse.getWhen(), ex | InputEvent.META_DOWN_MASK,
                                mouse.getX(), mouse.getY(), mouse.getClickCount(), mouse.isPopupTrigger(), mouse.getButton());
                    }
                }
                super.dispatchEvent(event);
            }
        });
    }

    // Loads the gamepack's own classes first, except the shared signlink ones, which come from the loader.
    static final class GameLoader extends URLClassLoader {
        private final Set<String> shared;

        GameLoader(File jar, Set<String> shared, ClassLoader parent) throws Exception {
            super(new URL[] {jar.toURI().toURL()}, parent);
            this.shared = shared;
        }

        @Override
        protected synchronized Class<?> loadClass(String name, boolean resolve) throws ClassNotFoundException {
            if (shared.contains(name)) {
                return getParent().loadClass(name);
            }
            Class<?> c = findLoadedClass(name);
            if (c == null) {
                try {
                    c = findClass(name);
                } catch (ClassNotFoundException e) {
                    c = getParent().loadClass(name);
                }
            }
            if (resolve) {
                resolveClass(c);
            }
            return c;
        }
    }
}
