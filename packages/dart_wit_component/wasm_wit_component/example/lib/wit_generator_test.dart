import 'dart:convert';
import 'dart:io';

import 'package:dart_style/dart_style.dart';
import 'package:test/test.dart';
import 'package:wasm_wit_component/generator.dart';
// ignore: implementation_imports
import 'package:wasm_wit_component/src/generate_cli.dart';
import 'package:wasm_wit_component/wasm_wit_component.dart';
import 'package:wasm_wit_component_example/host_wit_generation.dart';
import 'package:wasm_wit_component_example/test_utils.dart';

final _formatter = DartFormatter();

void witDartGeneratorTests({Future<Directory> Function()? getDirectory}) {
  group('wit generator', () {
    test('generate cli', testOn: 'windows || mac-os || linux', () async {
      if (Platform.isAndroid || Platform.isIOS) return;

      final root = getRootDirectory();
      final base = root.uri.resolve('packages/dart_wit_component/');

      final output = File.fromUri(
        base.resolve('wasm_wit_component/test/temp/generator.dart'),
      );
      try {
        // 'dart run wasm_wit_component/bin/generate.dart wit/dart-wit-generator.wit wasm_wit_component/lib/src/generator.dart'
        await generateCli([
          base.resolve('wit/dart-wit-generator.wit').toFilePath(),
          output.path,
        ]);

        final content = await output.readAsString();
        final formatted = _formatter.format(content);
        final expected = await File.fromUri(
          base.resolve('wasm_wit_component/lib/src/generator.dart'),
        ).readAsString();
        expect(formatted, expected.replaceAll('\r\n', '\n'));
      } finally {
        if (output.existsSync()) {
          output.deleteSync();
        }
      }
    });

    group('cli args', () {
      test('--no-default', () {
        const pathToWit = 'wit/file.wit';
        const pathToDartFile = 'lib/file.dart';
        final args = GeneratorCLIArgs.fromArgs(
          [pathToWit, pathToDartFile, '--no-default', '--json-serialization'],
        );

        expect(
          args,
          const GeneratorCLIArgs(
            dartFilePath: pathToDartFile,
            watch: false,
            witInputPath: pathToWit,
            config: WitGeneratorConfig(
              inputs: FileSystemPaths(inputPath: pathToWit),
              jsonSerialization: true,
              copyWith_: false,
              equalityAndHashCode: false,
              toString_: false,
              fileHeader: null,
              generateDocs: false,
              int64Type: Int64TypeConfig.bigInt,
              useNullForOption: true,
              requiredOption: false,
              typedNumberLists: true,
              asyncWorker: false,
              sameClassUnion: true,
            ),
          ),
        );
      });

      test('--no-copy-with --json-serialization=false', () {
        const pathToWit = 'wit/file.wit';
        final args = GeneratorCLIArgs.fromArgs(
          [
            pathToWit,
            '--no-copy-with',
            '--watch',
            '--json-serialization=false',
          ],
        );

        expect(
          args,
          const GeneratorCLIArgs(
            dartFilePath: null,
            watch: true,
            witInputPath: pathToWit,
            config: WitGeneratorConfig(
              inputs: FileSystemPaths(inputPath: pathToWit),
              jsonSerialization: false,
              copyWith_: false,
              equalityAndHashCode: true,
              toString_: true,
              fileHeader: null,
              generateDocs: true,
              int64Type: Int64TypeConfig.bigInt,
              useNullForOption: true,
              requiredOption: false,
              typedNumberLists: true,
              asyncWorker: false,
              sameClassUnion: true,
            ),
          ),
        );
      });

      test('error messages', () {
        void hasError(List<String> args, String error) {
          expect(
            () => GeneratorCLIArgs.fromArgs(args),
            throwsA(predicate((e) => e.toString().contains(error))),
          );
        }

        hasError(
          ['wit/file.wit', '-no-copy-with'],
          'Invalid argument (1, -no-copy-with). Should be --<name>',
        );
        hasError(
          ['file.wit', '--no-copy-with', '--copy-with'],
          'Duplicate argument (2, --copy-with).',
        );
        hasError(
          ['/wit/file.wit', '--watch', '--copy-with=no'],
          'Invalid argument (2, --copy-with=no). Should be true or false.',
        );
        hasError(
          ['--copy-with=true'],
          'Missing positional argument `witInputPath`.',
        );
      });
    });

    const hostWitContents = '''
package host-namespace:host-pkg

world host {
  import print: func(msg: string)

  record record-test {
    a: u32,
    b: string,
    c: float64,
  }

  export run: func()
  export get: func() -> record-test
  export map: func(rec: record-test) -> record-test
  export map-i: func(rec: record-test, i: float32) -> record-test
  export receive-i: func(rec: record-test, i: float32)
}
''';

    test('file system input', () async {
      const isWeb = identical(0, 0.0);
      final String witPath;
      if (!isWeb && getDirectory != null) {
        final dir = await getDirectory();
        final file = File(
          dir.uri.resolve('host.wit').toFilePath(windows: Platform.isWindows),
        )
          ..createSync(recursive: true)
          ..writeAsStringSync(hostWitContents);
        addTearDown(file.deleteSync);

        witPath = file.path;
      } else {
        witPath = isWeb
            ? 'host/host.wit'
            : getRootDirectory()
                .uri
                .resolve(
                  'packages/dart_wit_component/wasm_wit_component/example/lib/host.wit',
                )
                .toFilePath();
      }
      final wasiConfig = wasiConfigFromPath(
        witPath,
        webBrowserFileSystem: {
          if (isWeb)
            'host': WasiDirectory({
              'host.wit': WasiFile(
                const Utf8Encoder().convert(hostWitContents),
              )
            }),
        },
      );
      final g = await createDartWitGenerator(wasiConfig: wasiConfig);
      final inputs = FileSystemPaths(
        inputPath: isWeb
            ? witPath
            : '${wasiConfig.preopenedDirs.first.wasmGuestPath}host.wit',
      );

      _validateHostResult(g, inputs);
    });

    test('in memory input', () async {
      final g = await createDartWitGenerator(
        wasiConfig: const WasiConfig(
          preopenedDirs: [],
          webBrowserFileSystem: {},
        ),
      );
      const inputs = InMemoryFiles(
        worldFile: WitFile(
          path: 'host.wit',
          contents: hostWitContents,
        ),
        pkgFiles: [],
      );

      _validateHostResult(g, inputs);
    });
  });
}

void _validateHostResult(DartWitGeneratorWorld g, WitGeneratorInput inputs) {
  final result = g.generate(
    config: defaultGeneratorConfig(inputs: inputs),
  );

  switch (result) {
    case Err(:final error):
      throw Exception(error);
    case Ok(ok: final file):
      final formatted = _formatter.format(file.contents);

      expect(formatted, hostWitDartOutput);
      expect(file.path, endsWith('host.wit'));
  }
}
