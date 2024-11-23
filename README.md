## resvg-jni

Use Resvg to render static SVG files instead of Apache Batik

*[resvg](https://github.com/linebender/resvg)* is an [SVG](https://en.wikipedia.org/wiki/Scalable_Vector_Graphics) rendering library.

It can be used as a Rust library, as a C library, and as a CLI application to render static SVG files.

The core idea is to make a fast, small, portable SVG library with the goal to support the whole SVG spec.

## How to use:
````java
 public static void main(String[] args) throws IOException {
        //工作目录 working dir
        var options = new ResvgJNI.RenderOptions("/workDir");
        //设置字体目录 set fonts dir
        options.LoadFontsDir("/workDir/fonts");
        var renderer = new ResvgJNI.Renderer(options);
        
        //Load SVG file with byte[]
        var inputFilePath = "/workDir/static/scorePanelDarkmode.svg";
        var outputFilePath = "output.png";

        var svgData = Files.readString(Path.of(inputFilePath));
    
        try {
            var data = renderer.RenderPng(svgData);
            Files.write(Path.of(outputFilePath), data);
        } catch (Exception e) {
            System.out.println(e);
        }
    }
````
code mainly developed by [Zh_jk](https://github.com/fantasyzhjk)
