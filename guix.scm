;;; SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
;;; SPDX-License-Identifier: GPL-3.0-or-later

(use-modules (gnu packages machine-learning)
             (gnu packages python-build)
             (gnu packages speech)
             (gnu packages wine)
             (guix build-system meson)
             (guix build-system pyproject)
             (guix gexp)
             (guix git-download)
             (guix packages)
             ((guix licenses) #:prefix license:)
             (guix utils))

(define-public freetrackclient64
  (package
   (name "freetrackclient64")
   (version "1.0.0.0")
   (source (local-file "wine/freetrackclient" "freetrackclient"
                       #:recursive? #t
                       #:select? (git-predicate (dirname (current-filename)))))
   (build-system meson-build-system)
   (arguments
    (list #:target "x86_64-w64-mingw32"
          ;; XXX: FTGetData returns FALSE on build environment, should at
          ; least return TRUE once with data filled with zeroes.
          #:tests? #f
          #:phases
          #~(modify-phases %standard-phases
              (add-before 'check 'create-wineprefix
                (lambda* (#:key tests? #:allow-other-keys)
                  (when tests?
                    (let ((wineprefix (string-append (getcwd) "/.wine"))
                          (home (string-append (getcwd) "/.home")))
                      (mkdir-p wineprefix)
                      (mkdir-p home)
                      (setenv "HOME" home)
                      (setenv "WINEPREFIX" wineprefix)
                      (setenv "WINEDEBUG" "-all")
                      (invoke "wineboot" "--init"))))))))
   (native-inputs (list wine64))
   (home-page "https://github.com/jeandudey/aurascan")
   (synopsis "FreeTrack protocol dynamic library for Windows")
   (description "This provides a @acronym{DLL, Dynamic Link Library} for the
FreeTrack protocol.")
   (license (list license:expat license:isc))))

(define-public freetrackclient
  (package
    (inherit freetrackclient64)
    (name "freetrackclient")
    (arguments
     (substitute-keyword-arguments (package-arguments freetrackclient64)
       ((#:target _ #f) "i686-w64-mingw32")))
    (native-inputs (list wine))))

(define-public python-sixdrepnet360
  (package
    (name "python-sixdrepnet360")
    (version "0.1.0")
    (source (local-file "models/sixdrepnet360" "sixdrepnet360"
                        #:recursive? #t
                        #:select? (git-predicate (dirname (current-filename)))))
    (build-system pyproject-build-system)
    (arguments
     (list #:build-backend "setuptools.build_meta"
           ;; NOTE: No tests.
           #:tests? #f))
    (native-inputs
     (list python-setuptools))
    (propagated-inputs
     (list onnx
           python-onnxscript
           python-pytorch
           python-torchvision))
    (home-page "https://github.com/jeandudey/aurascan")
    (synopsis "Head pose estimation machine vision neural network model")
    (description "This package provides a Python implementation of the 6DRepNet360
machine vision neural network model for estimating the head pose of a 224x224 image
of a face.")
    (license license:expat)))

(define-public freetrackwinebridge64
  (package
    (name "freetrackwinebridge64")
    (version "0.1.0")
    (source (local-file "wine/freetrackwinebridge" "freetrackwinebridge"
                        #:recursive? #t
                        #:select? (git-predicate (dirname (current-filename)))))
    (build-system meson-build-system)
    (arguments
     (list #:configure-flags #~(list "--cross-file=../source/cross.txt")
           #:phases
           #~(modify-phases %standard-phases
               (add-before 'configure 'cross-file
                 (lambda _
                   (call-with-output-file "cross.txt"
                     (lambda (port)
                       (format port "\
[binaries]
c = 'winegcc'
cpp = 'wineg++'
ar = 'ar'
strip = 'strip'
pkg-config = 'pkg-config'

[properties]
need_exe_wrapper = true

[built-in options]
c_args = ['-I~a']

[host_machine]
system = 'linux'
cpu_family = 'x86_64'
cpu = 'x86_64'
endian = 'little'
" (string-append #$(this-package-native-input "freetrackclient64") "/include/wine/wine/windows")))))))))
    (native-inputs
     (list freetrackclient64
           (wine-for-system)))
    (home-page "https://github.com/jeandudey/aurascan")
    (synopsis "Bridge between for Wine programs using FreeTrack")
    (description "This package provides a bridge for the FreeTrack protocol
for Wine to be able to send head pose and location information from POSIX
operating systems to applications using FreeTrack on Wine.  Applications can
write tracking information to a POSIX shared memory.")
    ;; TODO: Choose license.
    (license #f)))

(list freetrackclient
      freetrackclient64
      freetrackwinebridge64
      python-sixdrepnet360)
