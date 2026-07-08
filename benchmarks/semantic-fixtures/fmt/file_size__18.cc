# ID: src/os.cc:246
// Returns the size in bytes of the file referred to by descriptor fd_.
auto file_length(int fd_) -> long long {
  using Stat = struct stat;
  Stat file_stat = Stat();
  if (FMT_POSIX_CALL(fstat(fd_, &file_stat)) == -1)
    FMT_THROW(system_error(errno, FMT_STRING("cannot get file attributes")));
  static_assert(sizeof(long long) >= sizeof(file_stat.st_size),
                "return type of file::size is not large enough");
  return file_stat.st_size;
}
