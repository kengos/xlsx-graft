# frozen_string_literal: true

require "fiddle"

module XlsxGraft
  # Finds and opens the engine's shared library (libxlsx_graft_capi).
  #
  # Looked up in this order, and the first one set wins (a set path that does
  # not load raises rather than falls through):
  #
  #   1. the XLSX_GRAFT_LIBRARY environment variable
  #   2. the `path:` argument
  #   3. the copy shipped inside the gem, under lib/xlsx_graft/native/
  #
  # The environment comes first because where the engine lives is the
  # deployment's decision, and it has to be able to override application code.
  # Nothing here downloads anything.
  class Library
    ENV_VAR = "XLSX_GRAFT_LIBRARY"

    # The file name on each platform. Windows has no `lib` prefix.
    NAMES = %w[libxlsx_graft_capi.so libxlsx_graft_capi.dylib xlsx_graft_capi.dll].freeze

    PACKAGED_DIR = File.expand_path("native", __dir__)

    attr_reader :path

    class << self
      # @param path [String, nil] where the library is, when the environment
      #   does not say
      # @param env [#[]] the environment to read XLSX_GRAFT_LIBRARY from
      # @return [Library]
      # @raise [LibraryNotFound] when no library is found or it cannot be loaded
      def open(path: nil, env: ENV)
        found = presence(env[ENV_VAR]) || presence(path) || packaged
        raise LibraryNotFound, install_hint("no engine library was found") unless found

        new(found, handle(found))
      end

      private

      # An empty value counts as unset: dlopen("") would open the running
      # program itself rather than fail.
      def presence(value)
        value unless value.nil? || value.empty?
      end

      def packaged
        NAMES.map { |name| File.join(PACKAGED_DIR, name) }.find { |file| File.file?(file) }
      end

      # Never closed: dlclose of a Rust library that has run thread-local
      # destructors is not safe, and opening the same path again only raises
      # the loader's reference count.
      def handle(path)
        Fiddle::Handle.new(path).tap(&:disable_close)
      rescue Fiddle::DLError => e
        raise LibraryNotFound, install_hint("#{path} could not be loaded (#{e.message})")
      end

      def install_hint(reason)
        <<~MESSAGE.strip
          #{reason}.

          This gem never downloads the engine. Point it at one of these ways:
            * set #{ENV_VAR} to the path of a libxlsx_graft_capi library
            * pass library: "/path/to/libxlsx_graft_capi.so"
            * put the library in #{PACKAGED_DIR}
        MESSAGE
      end
    end

    def initialize(path, handle)
      @path = path
      @handle = handle
    end

    # A function the library exports. The types are always given: Fiddle's
    # default return type is a C int, which would cut a pointer in half.
    #
    # @return [Fiddle::Function]
    # @raise [LibraryNotFound] when the library does not export `name`
    def function(name, args, returns)
      Fiddle::Function.new(@handle[name], args, returns)
    rescue Fiddle::DLError => e
      raise LibraryNotFound, "#{@path} exports no `#{name}` (#{e.message})"
    end
  end
end
