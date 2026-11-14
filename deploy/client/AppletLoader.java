import java.applet.Applet;
import java.applet.AppletContext;
import java.applet.AppletStub;
import java.applet.AudioClip;
import java.awt.Dimension;
import java.awt.Frame;
import java.awt.Graphics;
import java.awt.Image;
import java.awt.event.WindowAdapter;
import java.awt.event.WindowEvent;
import java.io.InputStream;
import java.net.URI;
import java.net.URL;
import java.net.URLClassLoader;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.Collections;
import java.util.Enumeration;
import java.util.HashMap;
import java.util.Iterator;
import java.util.Map;

/**
 * Runs a client from a world's jav_config.ws, as a browser would: the config's params, the jar served beside
 * it, and the config's codebase (which the client takes its server host from). A client that is not itself an
 * applet (742's) is handed a host applet through supplyApplet, as Jagex's loader did. Arguments after the URL
 * override params, e.g. {@code 18=2 8=2} for 742's world and lobby ids on node 11.
 *
 * <pre>java --add-opens=java.base/java.lang=ALL-UNNAMED -Xmx512m deploy/client/AppletLoader.java http://127.0.0.1:8080/jav_config.ws</pre>
 */
public final class AppletLoader implements AppletStub, AppletContext {
    private final Map<String, String> config = new HashMap<>();
    private final Map<String, String> params = new HashMap<>();
    private Applet applet;

    public static void main(String[] args) throws Exception {
        String url = args.length > 0 ? args[0] : "http://127.0.0.1:8080/jav_config.ws";
        new AppletLoader().run(URI.create(url).toURL(), Arrays.copyOfRange(args, Math.min(1, args.length), args.length));
    }

    private static void call(Object target, String name, Class<?> type, Object arg) {
        try {
            if (type == null) {
                target.getClass().getMethod(name).invoke(target);
            } else {
                target.getClass().getMethod(name, type).invoke(target, arg);
            }
        } catch (ReflectiveOperationException e) {
            throw new IllegalStateException(name, e);
        }
    }

    /** The applet a non-applet client draws into. */
    private static final class Host extends Applet {
        private final Object game;

        Host(Object game) {
            this.game = game;
        }

        @Override
        public void paint(Graphics g) {
            call(game, "paint", Graphics.class, g);
        }

        @Override
        public void update(Graphics g) {
            call(game, "update", Graphics.class, g);
        }
    }

    private void run(URL configUrl, String[] overrides) throws Exception {
        String text;
        try (InputStream in = configUrl.openStream()) {
            text = new String(in.readAllBytes(), StandardCharsets.ISO_8859_1);
        }
        for (String line : text.split("\r?\n")) {
            int eq = line.indexOf('=');
            if (eq < 0) {
                continue;
            }
            String key = line.substring(0, eq);
            String value = line.substring(eq + 1);
            if (key.equals("param")) {
                put(value);
            } else {
                config.put(key, value);
            }
        }
        for (String override : overrides) {
            put(override);
        }
        URL jar = new URL(configUrl, "client.jar");
        URLClassLoader loader = new URLClassLoader(new URL[] {jar}, AppletLoader.class.getClassLoader());
        String main = config.getOrDefault("initial_class", "client.class").replace(".class", "");
        Object game = loader.loadClass(main).getDeclaredConstructor().newInstance();
        applet = game instanceof Applet a ? a : new Host(game);
        applet.setStub(this);

        Frame frame = new Frame(config.getOrDefault("title", "RuneScape"));
        applet.setPreferredSize(new Dimension(Integer.parseInt(config.getOrDefault("applet_minwidth", "1024")),
                Integer.parseInt(config.getOrDefault("applet_minheight", "768"))));
        frame.add(applet);
        frame.pack();
        frame.setLocationRelativeTo(null);
        frame.addWindowListener(new WindowAdapter() {
            @Override
            public void windowClosing(WindowEvent e) {
                if (game == applet) {
                    applet.stop();
                    applet.destroy();
                } else {
                    call(game, "stop", null, null);
                }
                System.exit(0);
            }
        });
        frame.setVisible(true);
        if (game == applet) {
            applet.init();
            applet.start();
        } else {
            call(game, "supplyApplet", Applet.class, applet);
            call(game, "init", null, null);
            call(game, "start", null, null);
        }
    }

    private void put(String param) {
        int split = param.indexOf('=');
        if (split > 0) {
            params.put(param.substring(0, split), param.substring(split + 1));
        }
    }

    @Override
    public boolean isActive() {
        return true;
    }

    @Override
    public URL getDocumentBase() {
        return getCodeBase();
    }

    @Override
    public URL getCodeBase() {
        try {
            String base = config.getOrDefault("codebase", "http://127.0.0.1/");
            return URI.create(base.endsWith("/") ? base : base + "/").toURL();
        } catch (Exception e) {
            throw new IllegalStateException(e);
        }
    }

    @Override
    public String getParameter(String name) {
        return params.get(name);
    }

    @Override
    public AppletContext getAppletContext() {
        return this;
    }

    @Override
    public void appletResize(int width, int height) {
    }

    @Override
    public AudioClip getAudioClip(URL url) {
        return null;
    }

    @Override
    public Image getImage(URL url) {
        return null;
    }

    @Override
    public Applet getApplet(String name) {
        return applet;
    }

    @Override
    public Enumeration<Applet> getApplets() {
        return Collections.enumeration(Collections.singletonList(applet));
    }

    @Override
    public void showDocument(URL url) {
    }

    @Override
    public void showDocument(URL url, String target) {
    }

    @Override
    public void showStatus(String status) {
    }

    @Override
    public void setStream(String key, InputStream stream) {
    }

    @Override
    public InputStream getStream(String key) {
        return null;
    }

    @Override
    public Iterator<String> getStreamKeys() {
        return Collections.emptyIterator();
    }
}
