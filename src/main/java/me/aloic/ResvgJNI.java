package me.aloic;

import java.io.*;
import java.lang.ref.Cleaner;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.Comparator;

class ResvgJNI {
    private static final Path tempDir;
    static {
        try {
            String osName = System.getProperty("os.name").toLowerCase();
            String libraryName;
            if (osName.contains("win")) {
                libraryName = "resvg_jni.dll";
            } else if (osName.contains("nix") || osName.contains("nux") || osName.contains("aix")) {
                libraryName = "libresvg_jni.so";
            } else {
                throw new UnsupportedOperationException("Unsupported OS: " + osName);
            }
            String workingDir = System.getenv("RESVG_DIR");
            if (workingDir == null || workingDir.isEmpty()) {
                workingDir = String.valueOf(Files.createTempDirectory("rust_libs"));
            }
            tempDir = Path.of(workingDir);
            tempDir.toFile().deleteOnExit();

            Path tempLib = tempDir.resolve(libraryName);
            try (InputStream in = ResvgJNI.class.getResourceAsStream("/lib/" + libraryName)) {
                if (in == null) {
                    throw new FileNotFoundException("Library " + libraryName + " not found in JAR.");
                }
                Files.copy(in, tempLib, StandardCopyOption.REPLACE_EXISTING);
            }
            System.load(tempLib.toAbsolutePath().toString());
            Runtime.getRuntime().addShutdownHook(new Thread(() -> {
                try {
                    Files.walk(tempDir)
                            .sorted(Comparator.reverseOrder())
                            .map(Path::toFile)
                            .forEach(File::delete);
                } catch (IOException e) {
                    e.printStackTrace(); // 或记录日志
                }
            }));

        } catch (Exception e) {
            throw new RuntimeException("Failed to load the native library", e);
        }
    }

    private static native long RenderOptionsNew(String resourcePath);
    /// 0: OptimizeSpeed
    /// 1: CrispEdges
    /// 2: GeometricPrecision (default)
    private static native void RenderOptionsSetShapeRendering(long ptr, int render_type);
    /// 0: OptimizeSpeed
    /// 1: OptimizeLegibility (default)
    /// 2: GeometricPrecision
    private static native void RenderOptionsSetTextRendering(long ptr, int render_type);
    /// 0: OptimizeQuality (default)
    /// 1：OptimizeSpeed
    private static native void RenderOptionsSetImageRendering(long ptr, int render_type);
    private static native void RenderOptionsLoadSystemFonts(long ptr);
    private static native void RenderOptionsLoadFont(long ptr, String fontPath);
    private static native void RenderOptionsLoadFontsDir(long ptr, String fontDirPath);
    private static native void RenderOptionsDestroy(long ptr);
    private static native byte[] convertToPNG(long optionsPtr, String svgData, float scale);
    private static native byte[] convertToJPG(long optionsPtr, String svgData, float scale);

    public static class Renderer {
        private RenderOptions options;

        public Renderer(RenderOptions options) {
            this.options = options;
        }

        public byte[] RenderPng(String svgData, float scale) {
            return convertToPNG(options.GetContext(), svgData, scale);
        }

        public byte[] RenderPng(String svgData) {
            return RenderPng(svgData, 1.0f);
        }

        public byte[] RenderPng(InputStream svgInputStream, float scale) throws IOException {
            String svgData = readInputStreamToString(svgInputStream);
            return RenderPng(svgData, scale);
        }

        public byte[] RenderPng(InputStream svgInputStream) throws IOException {
            return RenderPng(svgInputStream, 1.0f);
        }

        public void RenderPng(InputStream svgInputStream, OutputStream pngOutputStream, float scale) throws IOException {
            byte[] pngData = RenderPng(svgInputStream, scale);
            pngOutputStream.write(pngData);
        }

        public void RenderPng(InputStream svgInputStream, OutputStream pngOutputStream) throws IOException {
            RenderPng(svgInputStream, pngOutputStream, 1.0f);
        }

        public byte[] RenderJpg(String svgData, float scale) {
            return convertToJPG(options.GetContext(), svgData, scale);
        }

        public byte[] RenderJpg(String svgData) {
            return RenderJpg(svgData, 1.0f);
        }

        public byte[] RenderJpg(InputStream svgInputStream, float scale) throws IOException {
            String svgData = readInputStreamToString(svgInputStream);
            return RenderJpg(svgData, scale);
        }

        public byte[] RenderJpg(InputStream svgInputStream) throws IOException {
            return RenderJpg(svgInputStream, 1.0f);
        }

        public void RenderJpg(InputStream svgInputStream, OutputStream JpgOutputStream, float scale) throws IOException {
            byte[] JpgData = RenderJpg(svgInputStream, scale);
            JpgOutputStream.write(JpgData);
        }

        public void RenderJpg(InputStream svgInputStream, OutputStream JpgOutputStream) throws IOException {
            RenderJpg(svgInputStream, JpgOutputStream, 1.0f);
        }

        // 工具方法：将 InputStream 转换为字符串
        private String readInputStreamToString(InputStream inputStream) throws IOException {
            return new String(inputStream.readAllBytes());
        }
    }

    public static class RenderOptions {
        private static final Cleaner cleaner = Cleaner.create();
        private final long context; // 保存 Rust 结构体的指针
        @SuppressWarnings("unused")
        private final Cleaner.Cleanable cleanable;

        public RenderOptions(String resourcePath) {
            this.context = RenderOptionsNew(resourcePath);
            this.cleanable = cleaner.register(this, new RustReleaser(context));
        }

        public long GetContext() {
            return context;
        }

        public void SetShapeRendering(int render_type) {
            RenderOptionsSetShapeRendering(context, render_type);
        }

        public void SetTextRendering(int render_type) {
            RenderOptionsSetTextRendering(context, render_type);
        }

        public void SetImageRendering(int render_type) {
            RenderOptionsSetImageRendering(context, render_type);
        }

        public void LoadSystemFonts() {
            RenderOptionsLoadSystemFonts(context);
        }

        public void LoadFont(String fontPath) {
            RenderOptionsLoadFont(context, fontPath);
        }

        public void LoadFontsDir(String fontPath) {
            RenderOptionsLoadFontsDir(context, fontPath);
        }

        // 静态内部类，负责释放 Rust 资源
        private static class RustReleaser implements Runnable {
            private final long rustPtr;

            RustReleaser(long rustPtr) {
                this.rustPtr = rustPtr;
            }

            @Override
            public void run() {
                RenderOptionsDestroy(rustPtr);
            }
        }
    }
}
