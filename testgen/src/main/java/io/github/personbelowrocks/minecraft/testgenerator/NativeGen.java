package io.github.personbelowrocks.minecraft.testgenerator;

import org.apache.commons.io.FileUtils;
import org.apache.commons.io.IOUtils;
import org.bukkit.Bukkit;

import java.io.File;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.Date;
import java.util.logging.Logger;

public class NativeGen {
    private final double state;
    private static final Logger logger = Bukkit.getLogger();
    private static final String LIB_BIN = "/lib-bin/";
    private static final String LIB_NAME = "rustgen";

    /**
     * When packaged into JAR extracts DLLs, places these into
     */
    public static String getLibPath() {
        // we need to put both DLLs to temp dir
        String path = "AC_" + new Date().getTime();
        return createLibPath(path, LIB_NAME);
    }

    /**
     * Puts library to temp dir and loads to memory
     */
    private static String createLibPath(String path, String name) {
        name = name + ".dll";
        try {
            // have to use a stream
            InputStream in = NativeGen.class.getResourceAsStream(LIB_BIN + name);
            // always write to different location
            File fileOut = new File(System.getProperty("java.io.tmpdir") + "/" + path + LIB_BIN + name);
            logger.info("Writing dll to: " + fileOut.getAbsolutePath());
            OutputStream out = FileUtils.openOutputStream(fileOut);
            IOUtils.copy(in, out);
            in.close();
            out.close();
            return fileOut.toString();
        } catch (Exception e) {
            logger.severe("epic fail loading DLL: " + e);
            throw new UnsatisfiedLinkError("oopsie!");
        }
    }

    public NativeGen(double state) {
        this.state = state;
    }
}
