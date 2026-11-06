"""Actual synthetic Core/probe subprocess IPC tests. No real Agent is started."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import unittest
import uuid

ROOT = Path(__file__).resolve().parents[1]
CORE = ROOT / "target/debug/agentguard-core.exe"
PROBE = ROOT / "target/debug/agentguard-probe.exe"


class SyntheticIPCTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if sys.platform != "win32":
            raise unittest.SkipTest("Windows named pipe tests require Windows")
        cls.core_hash = hashlib.sha256(CORE.read_bytes()).hexdigest()
        cls.probe_hash = hashlib.sha256(PROBE.read_bytes()).hexdigest()

    def scenario(self, mode, *, client_hash=None, server_pid=None, server_hash=None):
        pipe = "\\\\.\\pipe\\AgentGuard.Synthetic." + str(uuid.uuid4())
        core = subprocess.Popen([str(CORE), "serve-synthetic", pipe,
                                 client_hash or self.probe_hash], cwd=ROOT,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            probe = subprocess.run([str(PROBE), pipe, str(server_pid or core.pid),
                                    server_hash or self.core_hash, mode], cwd=ROOT,
                                   capture_output=True, timeout=20, encoding="utf-8")
            stdout, stderr = core.communicate(timeout=10)
            return probe, core.returncode, stdout.decode("utf-8"), stderr.decode("utf-8")
        finally:
            if core.poll() is None:
                core.kill()
                core.wait(timeout=5)

    def test_normal_request_stays_synthetic_and_unexecuted(self):
        client, status, output, error = self.scenario("valid")
        self.assertEqual((client.returncode, status), (0, 0), client.stderr + error)
        result = json.loads(client.stdout)["result"]
        self.assertEqual(result["status"], "validated_not_persisted")
        self.assertIs(result["executed"], False)
        self.assertIs(result["authorization_available"], False)

    def test_duplicate_source_is_not_a_new_operation(self):
        client, status, _, error = self.scenario("duplicate")
        self.assertEqual((client.returncode, status), (0, 0), client.stderr + error)
        self.assertEqual(json.loads(client.stdout)["result"]["status"], "duplicate")

    def test_changed_content_with_same_source_is_rejected(self):
        client, status, _, error = self.scenario("conflict")
        self.assertEqual(client.returncode, 0, client.stderr + error)
        self.assertEqual(json.loads(client.stdout)["error"]["code"], "source_conflict")

    def test_adapter_allow_and_os_identity_forgery_are_rejected(self):
        for mode in ("forged_allow", "forged_identity"):
            with self.subTest(mode=mode):
                client, status, _, error = self.scenario(mode)
                self.assertEqual(client.returncode, 0, client.stderr + error)
                self.assertEqual(json.loads(client.stdout)["error"]["code"], "protected_fields")

    def test_no_rpc_can_issue_execution_permission(self):
        client, _, _, error = self.scenario("evaluate")
        self.assertEqual(client.returncode, 0, client.stderr + error)
        self.assertEqual(json.loads(client.stdout)["error"]["code"], "authorization_unavailable")

    def test_connection_replay_and_wrong_challenge_are_rejected(self):
        for mode in ("replay", "wrong_challenge"):
            with self.subTest(mode=mode):
                client, _, _, error = self.scenario(mode)
                self.assertEqual(client.returncode, 0, client.stderr + error)
                self.assertEqual(json.loads(client.stdout)["error"]["code"], "connection_binding")

    def test_os_peer_image_is_pinned_on_server(self):
        client, status, _, error = self.scenario("valid", client_hash="0" * 64)
        self.assertNotEqual(status, 0)
        self.assertIn("peer_mismatch", error)
        self.assertNotEqual(client.returncode, 0)

    def test_client_checks_actual_server_pid_and_image(self):
        for override in ({"server_pid": 1}, {"server_hash": "0" * 64}):
            with self.subTest(override=override):
                client, _, _, _ = self.scenario("valid", **override)
                self.assertNotEqual(client.returncode, 0)
                self.assertIn("server_identity", client.stderr)

    def test_oversized_and_truncated_frames_fail_without_execution(self):
        for mode, code in (("oversized", "frame_limit"), ("truncated", "ipc_truncated")):
            with self.subTest(mode=mode):
                client, status, _, error = self.scenario(mode)
                self.assertEqual(client.returncode, 0, client.stderr)
                self.assertNotEqual(status, 0)
                self.assertIn(code, error)

    def test_idle_connection_expires(self):
        client, status, _, error = self.scenario("stall")
        self.assertNotEqual(status, 0)
        self.assertIn("ipc_timeout", error)

    def test_first_instance_prevents_second_core_from_claiming_name(self):
        pipe = "\\\\.\\pipe\\AgentGuard.Synthetic." + str(uuid.uuid4())
        core = subprocess.Popen([str(CORE), "serve-synthetic", pipe, self.probe_hash],
                                cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            # Wait for the first server's pipe without opening a client connection.
            import time
            import ctypes
            wait_pipe = ctypes.windll.kernel32.WaitNamedPipeW
            wait_pipe.argtypes = [ctypes.c_wchar_p, ctypes.c_uint32]
            wait_pipe.restype = ctypes.c_int
            deadline = time.monotonic() + 5
            while not wait_pipe(pipe, 10):
                if time.monotonic() > deadline:
                    self.fail("first Core did not create pipe")
                time.sleep(0.02)
            second = subprocess.run([str(CORE), "serve-synthetic", pipe, self.probe_hash],
                                    cwd=ROOT, capture_output=True, encoding="utf-8", timeout=10)
            self.assertNotEqual(second.returncode, 0)
            self.assertIn("pipe_create", second.stderr)
        finally:
            if core.poll() is None:
                core.kill()
            core.communicate(timeout=5)

    def test_nonlocal_or_unscoped_pipe_name_is_rejected(self):
        result = subprocess.run([str(CORE), "serve-synthetic", "\\\\remote\\pipe\\anything", self.probe_hash],
                                cwd=ROOT, capture_output=True, encoding="utf-8", timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("peer_identity", result.stderr)

    def test_core_cli_uses_the_same_strict_bounded_validation(self):
        import tempfile
        workspace = ROOT / ".work"
        workspace.mkdir(exist_ok=True)
        fixtures = json.loads((ROOT / "fixtures/p0.1.json").read_text(encoding="utf-8"))
        event = next(f["value"] for f in fixtures if f["id"] == "EV01")
        with tempfile.TemporaryDirectory(prefix="cli-input-", dir=workspace) as folder:
            path = Path(folder) / "input.json"
            examples = [(json.dumps(event), True),
                        (json.dumps(event)[:-1] + ',"event_id":"FAKE_SECRET"}', False),
                        (" " * 1_048_577, False)]
            for raw, valid in examples:
                with self.subTest(valid=valid, size=len(raw)):
                    path.write_text(raw, encoding="utf-8")
                    result = subprocess.run([str(CORE), "validate", "event-v2", str(path)],
                                            capture_output=True, encoding="utf-8", timeout=20)
                    self.assertEqual(result.returncode == 0, valid)
                    self.assertNotIn("FAKE_SECRET", result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
