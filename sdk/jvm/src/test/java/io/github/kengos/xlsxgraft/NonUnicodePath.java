// A Path whose absolute form holds an unpaired surrogate, as only a Windows file name can, over a
// file the default file system can read. The default file system makes no such path, so a test
// that needs one gives the package this: it is read through a file system of its own that opens the
// file it stands for.

package io.github.kengos.xlsxgraft;

import java.io.IOException;
import java.lang.reflect.Proxy;
import java.net.URI;
import java.nio.channels.SeekableByteChannel;
import java.nio.file.AccessMode;
import java.nio.file.CopyOption;
import java.nio.file.DirectoryStream;
import java.nio.file.FileStore;
import java.nio.file.FileSystem;
import java.nio.file.Files;
import java.nio.file.LinkOption;
import java.nio.file.OpenOption;
import java.nio.file.Path;
import java.nio.file.PathMatcher;
import java.nio.file.WatchService;
import java.nio.file.attribute.BasicFileAttributes;
import java.nio.file.attribute.FileAttribute;
import java.nio.file.attribute.FileAttributeView;
import java.nio.file.attribute.UserPrincipalLookupService;
import java.nio.file.spi.FileSystemProvider;
import java.util.Collections;
import java.util.Map;
import java.util.Set;

final class NonUnicodePath {

  private NonUnicodePath() {}

  /** A path named {@code "\ud800.xlsx"} that opens {@code file} (which need not exist). */
  static Path standingFor(Path file) {
    FileSystem system = new Opening(file);
    return (Path)
        Proxy.newProxyInstance(
            Path.class.getClassLoader(),
            new Class<?>[] {Path.class},
            (proxy, method, args) -> {
              switch (method.getName()) {
                case "toAbsolutePath":
                  return proxy;
                case "toString":
                  return "/\ud800.xlsx";
                case "getFileSystem":
                  return system;
                default:
                  throw new UnsupportedOperationException(method.getName());
              }
            });
  }

  private static final class Opening extends FileSystem {

    private final FileSystemProvider provider;

    Opening(Path file) {
      this.provider = new OpeningProvider(file);
    }

    @Override
    public FileSystemProvider provider() {
      return provider;
    }

    @Override
    public void close() {}

    @Override
    public boolean isOpen() {
      return true;
    }

    @Override
    public boolean isReadOnly() {
      return true;
    }

    @Override
    public String getSeparator() {
      return "/";
    }

    @Override
    public Iterable<Path> getRootDirectories() {
      return Collections.<Path>emptyList();
    }

    @Override
    public Iterable<FileStore> getFileStores() {
      return Collections.<FileStore>emptyList();
    }

    @Override
    public Set<String> supportedFileAttributeViews() {
      return Collections.<String>emptySet();
    }

    @Override
    public Path getPath(String first, String... more) {
      throw new UnsupportedOperationException();
    }

    @Override
    public PathMatcher getPathMatcher(String syntaxAndPattern) {
      throw new UnsupportedOperationException();
    }

    @Override
    public UserPrincipalLookupService getUserPrincipalLookupService() {
      throw new UnsupportedOperationException();
    }

    @Override
    public WatchService newWatchService() {
      throw new UnsupportedOperationException();
    }
  }

  // Opens the file the path stands for, whatever path it is asked for; nothing else.
  private static final class OpeningProvider extends FileSystemProvider {

    private final Path file;

    OpeningProvider(Path file) {
      this.file = file;
    }

    @Override
    public SeekableByteChannel newByteChannel(
        Path path, Set<? extends OpenOption> options, FileAttribute<?>... attrs)
        throws IOException {
      return Files.newByteChannel(file, options);
    }

    @Override
    public String getScheme() {
      return "nonunicode";
    }

    @Override
    public FileSystem newFileSystem(URI uri, Map<String, ?> env) {
      throw new UnsupportedOperationException();
    }

    @Override
    public FileSystem getFileSystem(URI uri) {
      throw new UnsupportedOperationException();
    }

    @Override
    public Path getPath(URI uri) {
      throw new UnsupportedOperationException();
    }

    @Override
    public DirectoryStream<Path> newDirectoryStream(
        Path dir, DirectoryStream.Filter<? super Path> filter) {
      throw new UnsupportedOperationException();
    }

    @Override
    public void createDirectory(Path dir, FileAttribute<?>... attrs) {
      throw new UnsupportedOperationException();
    }

    @Override
    public void delete(Path path) {
      throw new UnsupportedOperationException();
    }

    @Override
    public void copy(Path source, Path target, CopyOption... options) {
      throw new UnsupportedOperationException();
    }

    @Override
    public void move(Path source, Path target, CopyOption... options) {
      throw new UnsupportedOperationException();
    }

    @Override
    public boolean isSameFile(Path path, Path path2) {
      throw new UnsupportedOperationException();
    }

    @Override
    public boolean isHidden(Path path) {
      throw new UnsupportedOperationException();
    }

    @Override
    public FileStore getFileStore(Path path) {
      throw new UnsupportedOperationException();
    }

    @Override
    public void checkAccess(Path path, AccessMode... modes) {
      throw new UnsupportedOperationException();
    }

    @Override
    public <V extends FileAttributeView> V getFileAttributeView(
        Path path, Class<V> type, LinkOption... options) {
      throw new UnsupportedOperationException();
    }

    @Override
    public <A extends BasicFileAttributes> A readAttributes(
        Path path, Class<A> type, LinkOption... options) {
      throw new UnsupportedOperationException();
    }

    @Override
    public Map<String, Object> readAttributes(Path path, String attributes, LinkOption... options) {
      throw new UnsupportedOperationException();
    }

    @Override
    public void setAttribute(Path path, String attribute, Object value, LinkOption... options) {
      throw new UnsupportedOperationException();
    }
  }
}
