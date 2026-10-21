try {
  Write-Host "=== 1. Test Admin Login ==="
  $loginBody = '{"username":"rusdifn08","password":"121234aa"}'
  $adminRes = Invoke-RestMethod -Uri "http://localhost:3000/api/auth/login" -Method Post -Body $loginBody -ContentType "application/json"
  $token = $adminRes.data.token
  $headers = @{ Authorization = "Bearer $token" }
  Write-Host "Admin Login Success, Token obtained."

  Write-Host "`n=== 2. Submit Guest Feedback ==="
  $guestFb = @{
    category = "feature_request"
    feature_affected = "Koleksi Resep Manual Brew"
    title = "Tambahkan Resep Aeropress Inverted Champion"
    description = "Sangat bagus jika ada resep Aeropress inverted dengan rasio 1:14 untuk biji natural Ethiopia."
    rating = 5
    guest_name = "Barista Tamu"
    app_version = "1.0.0"
    device_info = "Android 14"
  } | ConvertTo-Json
  $fbRes1 = Invoke-RestMethod -Uri "http://localhost:3000/api/feedback" -Method Post -Body $guestFb -ContentType "application/json"
  Write-Host "Guest Feedback Submitted: ID =" $fbRes1.data.id

  Write-Host "`n=== 3. Submit Member Feedback (with Bearer Token) ==="
  $memberFb = @{
    category = "bug_report"
    feature_affected = "Brew Timer & Stopwatch"
    title = "Suara beep tidak bunyi di step 2"
    description = "Ketika masuk ke step kedua, audio beep terkadang terlambat 1 detik."
    rating = 4
    app_version = "1.0.0"
    device_info = "Windows Desktop"
  } | ConvertTo-Json
  $fbRes2 = Invoke-RestMethod -Uri "http://localhost:3000/api/feedback" -Method Post -Body $memberFb -ContentType "application/json" -Headers $headers
  Write-Host "Member Feedback Submitted: ID =" $fbRes2.data.id "User =" $fbRes2.data.username

  Write-Host "`n=== 4. Admin Get Feedbacks ==="
  $feedbacksRes = Invoke-RestMethod -Uri "http://localhost:3000/api/admin/feedbacks" -Method Get -Headers $headers
  Write-Host "Total Feedbacks fetched:" $feedbacksRes.data.Length

  Write-Host "`n=== 5. Admin Get Feedback Stats ==="
  $statsRes = Invoke-RestMethod -Uri "http://localhost:3000/api/admin/feedbacks/stats" -Method Get -Headers $headers
  Write-Host ($statsRes.data | ConvertTo-Json)

  Write-Host "`n=== 6. Admin Update Feedback Status ==="
  $updateBody = @{
    status = "in_progress"
    admin_notes = "Sedang diperbaiki oleh tim audio di update mendatang."
  } | ConvertTo-Json
  $updateUrl = "http://localhost:3000/api/admin/feedbacks/$($fbRes2.data.id)/status"
  $updatedRes = Invoke-RestMethod -Uri $updateUrl -Method Post -Body $updateBody -ContentType "application/json" -Headers $headers
  Write-Host "Updated Feedback Status:" $updatedRes.data.status "Notes:" $updatedRes.data.admin_notes

  Write-Host "`n=== 7. Admin Clean Up Test Feedbacks ==="
  Invoke-RestMethod -Uri "http://localhost:3000/api/admin/feedbacks/$($fbRes1.data.id)" -Method Delete -Headers $headers
  Invoke-RestMethod -Uri "http://localhost:3000/api/admin/feedbacks/$($fbRes2.data.id)" -Method Delete -Headers $headers
  Write-Host "Test feedbacks cleaned up!"

  Write-Host "`n>>> ALL FEEDBACK ENDPOINTS PASSED 100%! <<<"
} catch {
  Write-Error $_
}
