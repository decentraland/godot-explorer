package com.dclrn

import android.app.Application
import android.util.Log
import com.google.android.gms.common.ConnectionResult
import com.google.android.gms.common.GoogleApiAvailability
import com.google.android.play.core.integrity.IntegrityManagerFactory
import com.google.android.play.core.integrity.IntegrityTokenRequest
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlinx.coroutines.suspendCancellableCoroutine
import org.decentraland.core.host.AttestationFailure
import org.decentraland.core.host.AttestationService

/** The OS half of Android attestation; the core owns the session, the cache and the retries. */
class PlayIntegrityAttestation(private val app: Application) : AttestationService {
  override fun isSupported(): Boolean =
      try {
        GoogleApiAvailability.getInstance().isGooglePlayServicesAvailable(app) ==
            ConnectionResult.SUCCESS
      } catch (e: Throwable) {
        false
      }

  override suspend fun playIntegrityToken(requestHashB64url: String): String =
      suspendCancellableCoroutine { continuation ->
        fun fail(e: Throwable) {
          Log.w(TAG, "Play Integrity failed: ${e.message}", e)
          continuation.resumeWithException(
              AttestationFailure.Failed("${e.javaClass.simpleName}: ${e.message}"))
        }
        try {
          IntegrityManagerFactory.create(app)
              .requestIntegrityToken(
                  IntegrityTokenRequest.builder().setNonce(requestHashB64url).build())
              .addOnSuccessListener { response ->
                val token = response?.token().orEmpty()
                if (token.isEmpty()) {
                  fail(IllegalStateException("empty integrity token"))
                } else {
                  continuation.resume(token)
                }
              }
              .addOnFailureListener { fail(it) }
        } catch (e: Throwable) {
          fail(e)
        }
      }

  override suspend fun iosGenerateKey(): String =
      throw AttestationFailure.Failed("App Attest is iOS only")

  override suspend fun iosAttestKey(keyId: String, clientDataHash: ByteArray): String =
      throw AttestationFailure.Failed("App Attest is iOS only")

  private companion object {
    const val TAG = "Attestation"
  }
}
