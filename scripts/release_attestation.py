#!/usr/bin/env python3
"""Offline reviewer-authenticated release evidence, distinct from GitHub lab CI.

Two independent registered Ed25519 reviewers must sign the same, precisely
scoped production evidence bytes. No private key material is stored in git.
The trust registry must be independently approved/protected by maintainers.
This is NOT, by itself, authorization for a production deployment.
"""
from __future__ import annotations
import base64
import binascii
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile

SHA = re.compile(r"^[a-f0-9]{40}$")
ID = re.compile(r"^[a-z][a-z0-9_-]{2,64}$")
# Every production gate has concrete non-skippable acceptance scenarios.
# A signed proof of only three unrelated green checks cannot pass.
REQUIRED_SCENARIOS={
    "authenticated_host_transport":{
        "real_host_transport","actual_user_revocation",
        "connection_recovery","privilege_separation"},
    "ha_core_hacs_29_runtime":{
        "all_29_integrations","lifecycle_migrations",
        "upstream_marketplace","recovery_after_restart"},
    "hacs_github_oauth":{
        "real_github_oauth","token_renewal",
        "revocation_and_expiry","multiuser_isolation"},
    "apps_9_lifecycle":{
        "nine_real_supervisor_apps","install_update_uninstall",
        "persistent_state_integrity","negative_failure_cases"},
    "supervisor_apps_restore":{
        "real_app_data_restore","full_app_state_restore",
        "restart_and_ingress","generic_vendor_support"},
    "haos_backup_restore":{
        "fresh_os_restore","encrypted_archive",
        "data_integrity","complete_service_recovery"},
    "rollback_verified":{
        "deliberate_partial_failure","prior_data_identical",
        "prior_state_identical","rollback_under_outage"},
    "single_radio_owner":{
        "real_zigbee_owner","real_thread_matter_owner",
        "concurrent_controller_denial","crash_failover"},
    "ha_ws_reconnection":{
        "token_expiry","connection_loss",
        "long_running_soak","convergent_reconnect"},
    "production_ui_auth":{
        "real_user_sessions","per_app_rbac",
        "csrf_and_revocation","one_time_confirmation"},
    "native_drivers_services":{
        "physical_driver_matrix","usb_serial_bluetooth",
        "services_and_firmware","cross_platform_support"},
    "performance_full_stack":{
        "whole_stack_load","long_soak",
        "fault_recovery","baseline_resource_comparison"},
    "packaging_updates":{
        "production_signed_artifacts","existing_install_upgrade",
        "data_preservation","failed_upgrade_rollback"},
    "third_party_licenses":{
        "complete_spdx_sbom","all_source_licenses",
        "attribution_and_notices","independent_compliance_review"},
    "security_stress_fuzz":{
        "extended_full_surface_fuzz","penetration_assessment",
        "privilege_boundary_tests","remediation_retest"},
}

MAX_EVIDENCE_BYTES=64 * 1024
MAX_TRUST_BYTES=32 * 1024

def _unique_pairs(pairs):
    result={}
    for key,value in pairs:
        if key in result:
            raise ValueError("duplicate JSON key")
        result[key]=value
    return result

def _trust_roots(root:Path)->dict:
    path=root/"config/release-attestors.json"
    if path.is_symlink() or not path.is_file() or path.stat().st_size>MAX_TRUST_BYTES:
        raise ValueError("release reviewer trust roots absent or invalid")
    obj=json.loads(path.read_bytes(),object_pairs_hook=_unique_pairs)
    if set(obj)!={"schema_version","reviewers"} or obj["schema_version"]!=1:
        raise ValueError("invalid trust registry schema")
    reviewers=obj["reviewers"]
    if not isinstance(reviewers,list) or len(reviewers)<2 or len(reviewers)>32:
        raise ValueError("independent release reviewers not configured")
    keys={}
    fingerprints=set()
    for reviewer in reviewers:
        if not isinstance(reviewer,dict) or set(reviewer)!={"id","public_key_pem","active"}:
            raise ValueError("invalid reviewer")
        rid=reviewer["id"]
        pem=reviewer["public_key_pem"]
        if not isinstance(rid,str) or not ID.fullmatch(rid) or rid in keys:
            raise ValueError("duplicate or invalid reviewer")
        if not isinstance(pem,str) or len(pem)>2000 or not pem.startswith("-----BEGIN PUBLIC KEY-----"):
            raise ValueError("invalid public key")
        if type(reviewer["active"]) is not bool:
            raise ValueError("invalid activation status")
        if reviewer["active"]:
            fingerprint=hashlib.sha256(pem.encode("utf-8")).hexdigest()
            if fingerprint in fingerprints:
                raise ValueError("distinct reviewers share one public key")
            fingerprints.add(fingerprint)
            keys[rid]=pem
    if len(keys)<2:
        raise ValueError("fewer than two active independent reviewers")
    return keys

def _verify_signature(public_pem:str,evidence:bytes,signature:bytes)->bool:
    if len(signature)!=64:
        return False
    with tempfile.TemporaryDirectory(prefix="hars-review-") as td:
        base=Path(td)
        (base/"proof").write_bytes(evidence)
        (base/"signature").write_bytes(signature)
        (base/"public.pem").write_text(public_pem)
        result=subprocess.run([
            "openssl","pkeyutl","-verify","-pubin",
            "-inkey",str(base/"public.pem"),"-rawin",
            "-in",str(base/"proof"),"-sigfile",str(base/"signature")],
            capture_output=True,timeout=8,check=False)
        return result.returncode==0

def verified_release_artifact(root:Path,gate_id:str,artifact:dict)->bool:
    try:
        if not isinstance(artifact,dict) or set(artifact)!={"name","sha256","signatures"}:
            return False
        if artifact["name"]!=gate_id+".json":
            return False
        digest=artifact["sha256"]
        if not isinstance(digest,str) or not re.fullmatch(r"[a-f0-9]{64}",digest):
            return False
        path=root/"docs/certifications"/artifact["name"]
        if path.is_symlink() or not path.is_file() or path.stat().st_size>MAX_EVIDENCE_BYTES:
            return False
        raw=path.read_bytes()
        if not raw or hashlib.sha256(raw).hexdigest()!=digest:
            return False
        evidence=json.loads(raw,object_pairs_hook=_unique_pairs)
        if not isinstance(evidence,dict) or set(evidence)!={
            "schema_version","gate_id","scope","product_commit_sha",
            "validated_scenarios","independent_ci_runs","recovery_verified"
        }:
            return False
        if evidence["schema_version"]!=1 or evidence["gate_id"]!=gate_id:
            return False
        if evidence["scope"]!="production_end_to_end":
            return False
        sha=evidence["product_commit_sha"]
        if not isinstance(sha,str) or not SHA.fullmatch(sha):
            return False
        scenarios=evidence["validated_scenarios"]
        if (not isinstance(scenarios,list) or len(scenarios)<3
            or len(scenarios)>100 or len(set(map(str,scenarios)))!=len(scenarios)):
            return False
        if any(not isinstance(t,str) or not ID.fullmatch(t) for t in scenarios):
            return False
        mandatory=REQUIRED_SCENARIOS.get(gate_id)
        if mandatory is None or not mandatory.issubset(set(scenarios)):
            return False
        runs=evidence["independent_ci_runs"]
        if not isinstance(runs,list) or len(runs)<2 or len(runs)>50:
            return False
        if any(type(n) is not int or n<=0 for n in runs) or len(set(runs))!=len(runs):
            return False
        if evidence["recovery_verified"] is not True:
            return False
        keys=_trust_roots(root)
        signatures=artifact["signatures"]
        if not isinstance(signatures,list) or len(signatures)<2 or len(signatures)>32:
            return False
        seen=set()
        for review in signatures:
            if not isinstance(review,dict) or set(review)!={"reviewer","signature_b64"}:
                return False
            rid=review["reviewer"]
            if rid not in keys or rid in seen:
                return False
            seen.add(rid)
            encoded=review["signature_b64"]
            if not isinstance(encoded,str) or len(encoded)>128:
                return False
            try:
                signature=base64.b64decode(encoded,validate=True)
            except (ValueError,binascii.Error):
                return False
            if not _verify_signature(keys[rid],raw,signature):
                return False
        return len(seen)>=2
    except (ValueError,TypeError,KeyError,OSError,subprocess.TimeoutExpired,subprocess.SubprocessError):
        return False
